Learning Rust
=============

This project is first and foremost about learning Rust. I have been following
the development of the language since version 0.4 when I first learned about
it, and Rust no longer looks like then. This is not my first piece of Rust
code but trying random things isn't the same as starting a real project. After
hearing numerous times the rumor that Rust is a nice language for parsers, an
idea of a VCL preprocessor popped in my head and turned into a summer project.
The Varnish Configuration Language (VCL) is a simple language, and a good test
drive for a parser, although in this case the parsing is not comprehensive but
rather just enough to experiment with alternative syntaxes.

Takeways for vclpp 0.1
----------------------

To make this interesting, I set arbitrary design goals: no external crates,
and no backtracking. ``vclpp`` takes a PVCL input file and outputs VCL, and it
does so with multiple passes (one per alternative syntax).

Parsing
'''''''

So, is the rumor true? The answer is an astounding yes with the combination of
``enum`` and ``match``. The tokenizer consumes one character at a time, and
thanks to VCL's simplicity all I need to track is the previous character. That
takes care of all potential collisions (for example ``/`` for divisions or
``//`` and ``/*`` for comments).

Some interesting examples of parsing using those two primitives::

  match (self.lexeme.unwrap(), self.previous, c) {
      [...]
      (Delim(_), '/', '*') => (CComment, NeedsMore),
      (Delim(_), '/', '/') => (CxxComment, MayNeedMore),
      (Delim(_), '/', _) => (Delim('/'), PreviousReady),
      [...]
      (Name(_), '.', '.') => (self.error("invalid name"), Done),
      [...]
      (Name(d), _, '.') => (Name(d+1), NeedsMore),
      [...]
      (Integer, _, '.') => (Number, MayNeedMore),
      [...]
      (Number, _, '.') => (self.error("invalid number"), Done),
      [...]
  }

The first three illustrate the collision for tokens starting with the same
character. It almost reads "when the ``/`` delimiter is followed by an other
``/`` then it's a C++ comment and may need more characters to complete the
token".

The next example is a name (generic lexeme for symbols, identifiers or names)
that may contain dots (like ``req.url`` or ``req.http.cache-control``) where
two consecutive dots are a syntax error.

The name lexeme also keeps track of the number of dots, and the next example
shows how simple it is to do that.

The fourth example is the promotion of an integer lexeme to a decimal number,
this is again easy and using different lexemes in the ``enum`` enables simple
detection of syntax error in numbers as shown in the last example.

The entire state machine of the tokenizer fits in two ``match`` statements
that almost read like prose. Reputation well deserved.

Iterators
'''''''''

Rust, despite being a procedural language has a strong functional flavor, and
supposedly ``rustc`` and the LLVM backend will be capable most of time to turn
functional code based on iterators into efficient procedural loops. So to meet
the design goals, iterators were good candidates, and as a result were forced
in everywhere: the tokenizer takes a character iterator and is itself a token
iterator, while preprocessor passes consume token iterators which they are
themselves too. And of course all those iterators are lazy and all run in the
main (and only) thread.

So the result looks like this::

  file | tokenizer | pass 1 | ... | pass N | file

Due to the nature of files and their ability to fail operations, the input is
first read as a string before being turned into a character iterator and the
output is written to in a ``for`` loop.

Shared state
''''''''''''

In this setup tokens flow from the tokenizer and through the passes to then
be printed to the output. But some of the passes need to keep track of PVCL
tokens to later turn them into valid VCL tokens. Initially passes would derive
the ``Copy`` trait because it made everything so much simpler, but eventually
an ``RcToken`` type alias was introduced to wrap them into reference-counted
reference cells.

At this point readability takes a hit, despite the convenience offered by
type aliases. Presumably, not having to copy references between passes should
lower the memory footprint, reduce pressure on the allocator, and offer better
performances. I did not measure this and performance is not really important
for a preprocessor of this kind. ``vclpp`` seems to perform fine::

  for i in {1..1000}
  do
      varnishd -x builtin
  done | time -v ./src/vclpp >/dev/null

      Command being timed: "./src/vclpp"
      User time (seconds): 0.00
      System time (seconds): 0.01
      Percent of CPU this job got: 1%
      Elapsed (wall clock) time (h:mm:ss or m:ss): 0:01.45
      Average shared text size (kbytes): 0
      Average unshared data size (kbytes): 0
      Average stack size (kbytes): 0
      Average total size (kbytes): 0
      Maximum resident set size (kbytes): 7160
      Average resident set size (kbytes): 0
      Major (requiring I/O) page faults: 0
      Minor (reclaiming a frame) page faults: 1446
      Voluntary context switches: 1002
      Involuntary context switches: 0
      Swaps: 0
      File system inputs: 0
      File system outputs: 0
      Socket messages sent: 0
      Socket messages received: 0
      Signals delivered: 0
      Page size (bytes): 4096
      Exit status: 0

Most of the time spent is waiting for the thousand ``varnishd`` executions to
complete while the preprocessor is accumulating the input. No user should feel
like complaining regardless how large their code is.

This is only a while after that I realized I didn't need the RefCell in the
first place. When PVCL code is being rewritten to VCL, either tokens are left
untouched or synthetic tokens are created (tokens that aren't in the original
input). Mutability isn't actually needed, so legibility didn't need harm.

Ownership
'''''''''

Obviously a big topic in Rust, possibly the main one. That was most certainly
*the* selling point when I discovered Rust. It's a powerful tool for thinking
since ownership in the physical sense is something easy to grasp, applying it
to programming changes the perspective of code and resources management, and
not just when programming in Rust.

This is not my first exposure to ownership and borrowing, or even lifetimes,
but dealing with reference counting turned the table. I found an interesting
corner case in the compiler where two alternative syntaxes (how ironic!) for
the same thing (returning a value) don't yield the same results.

https://github.com/rust-lang/rust/issues/44019

Unfortunately it was closed with an arcane hint and understanding it is left
as an exercise to the reader. I personally don't see the whole picture yet.

Some syntax subtleties are also still beyond my understanding, one tough nut
to crack was difference between ``&`` and ``ref``::

  fn write(&mut self, buf: &[u8]) -> Result<usize> {
      match self {
          &mut Arg(ref mut bw) => bw.write(buf),
          &mut Def(ref mut bw) => bw.write(buf),
      }
  }

And for some reason when the compiler would yell at me and suggest adding
``ref`` or ``ref mut`` my mind automatically read ``&`` or ``&mut``...

Build system
''''''''''''

I already explained in the installation notes why I don't follow the cargo
cult, so this isn't the topic here. Rust has a very reach type system, and the
Rust compiler's borrow checker can even be considered a static analyzer. Error
messages are legion, and most of the time the Rust compiler can ``--explain``
the problems with great details. I found one case lacking an explanation for
an error I ran into.

https://github.com/rust-lang/rust/issues/43913

Another great thing is the ability to access environment variables at compile
time or use your own configuration via the ``cfg!`` macro or the ``#[cfg]``
decorator. I'm using this to make sure that all iterators are behaving sanely
and only return ``None`` after a bad token or a another ``None``, only when
running with ``kcov``. I also wanted a 100% line coverage, and pretend this
was a serious project, ``varnishtest`` proved to be a valuable test framework
for that.

One thing I don't like, is ``rustc`` complaining about (unused) dead code when
building the ``vcltok`` program. The code in question is used by ``vclpp`` but
those are warnings I can live with as long as they are confined to ``vcltok``.

Macro system
''''''''''''

Some people consider the lack of a macro system a good language feature, and
preprocessors like C's get a bad rap. In the C projects I work on, we use what
we call macro tables that are defined in a file, and may be included and turn
into different code in different places for different needs, including docs.

This is not possible in Rust to the extent of my understanding of macros, Rust
macros are a different breed altogether. The use of macros for safe compiled
print patterns is definitely a plus. The expressiveness of built-in macros
like ``panic``, ``unimplemented``, ``assert`` or ``unreachable`` makes it also
pleasant not having to reinvent them for simple uses. ``unreachable`` in
particular makes a good marker for exclusion from a code coverage report.

Still a beginner
''''''''''''''''

Learning Rust is a pleasant experience, however the learning curve can be both
steep and flat. I requested help twice on IRC, and always got helpful answers.
There are still many unknowns, but usually the documentation is so good that
even with no internet connection I tend to find what I'm looking for.

I found the hard way that the standard streams swallow bad descriptor errors,
and while I get the rationale I don't subscribe to the idea of silently
ignoring errors. Error handling is one of the best things offered by Rust,
with the high incentive to actually handle them, and the convenience of easily
converting them when they need to bubble up the layers.

One thing I wanted to do is to turn the passes into a vector::

  let passes = vec!(
      DeclarativeObject::new,
      RequestAuthority::new,
      VmodAlias::new,
      HeaderArray::new,
  );

  #[cfg(kcov)]
  shuffle(passes);

  for pass in passes {
      # build the preprocessor
  }

This way when running the test suite for code coverage, it could verify that
changing the order of the passes doesn't change the result. If this is even
possible (infinite recursive type warnings don't bode well) that's a challenge
for later.

It is time to let this code sleep a couple months and see whether it is still
readable after being swapped out of my mind's resident memory.
