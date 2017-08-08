Installation instructions
=========================

Unlike usual Rust code bases, ``vclpp`` uses the GNU build system in order to
integrate well with Varnish and especially ``varnishtest`` to run the test
suite.

Dependencies:

- Rust >= 1.19
- Varnish >= 5.1.3 (optional)
- pkg-config

It relies by default on system-wide installations of those tools, but can be
configured otherwise. For example, you can pick a specific Rust compiler::

  ./configure RUSTC=/path/to/rustc

To target a specific version of Varnish, use pkg-config::

  ./configure PKG_CONFIG_PATH=/opt/varnish/lib/pkgconfig

This will build ``vclpp`` for the Varnish installation in ``/opt/varnish``.
If `vclpp` should be installed installed alongside Varnish, pick a prefix::

  ./configure --prefix=/opt/varnish PKG_CONFIG_PATH=/opt/varnish/lib/pkgconfig

The default prefix is ``/usr/local``, and to learn about other configuration
options you can run ``./configure --help``. If Varnish is not available or not
recent enough, the test suite can be skipped with ``--without-tests``.

Once installed, ``vclpp`` has virtually no runtime dependencies.

Building from git
-----------------

When the sources are built directly from a git clone, the ``configure`` script
is missing and needs to be generated first. The ``bootstrap`` script does that
but also triggers the configuration. Arguments to the ``bootstrap`` script are
passed to the ``configure`` execution::

  ./bootstrap
  make
  sudo make install

Additional dependencies:

- autoconf >= 2.68
- automake >= 1.12
- rst2man

To target a specific version of Varnish, aclocal needs to be configured in
addition to pkg-config. Instead of the command line, it can be added to the
environment for convenience::

  export PKG_CONFIG_PATH=/opt/varnish/lib/pkgconfig
  export ACLOCAL_PATH=/opt/varnish/share/aclocal
  ./bootstrap --prefix=/opt/varnish
  make
  sudo make install

This will ensure that the correct autoconf macros are used during the build.

The cargo cult
--------------

Why is this not a standard Rust project besides the reasons outlined above?

I have been following Rust since version 0.4 and was immediately sold on the
idea. At the time though, the language looked nothing like Rust 1.0 but the
principles were there. "Following" is somewhat an overstatement, it is mostly
limited to the Rust blog, some podcasts that didn't seem to last long, and
RustConf videos that look interesting.

At some point, what I was hoping not to ever see happened: Cargo. The problem
is not Cargo, it's the unspoken law of programming languages that seems to
state that any popular language should grow a build system and confuse it with
package and dependency managers. Why is this a problem? Because it becomes
needlessly hard to implement polyglot projects like ``vclpp``.

But wait, ``vclpp`` is written in Rust, right? Or did I by some magic trick
manage to let your git implementation hide secret C files or something of the
sort? No, it's even more insidious: the other language is RST and its purpose
is to build manual pages.

Last time I checked the Cargo documentation (that is to say a long time ago
before I started this project) it wasn't trivial to do that. I believe it
involved a ``build.rs`` file for customization, and as a Rust beginner I find
this requirement somewhat ironic. Also writing Rust code to run external
commands at build-time doesn't sound compelling, if anything, it is the wrong
level of abstraction. Looking at the cargo man page, there were no hints of
progress in this area so I didn't even look at the rest of the docs. If I'm
wrong and those observations are stale, that's on me.

So what's the current solution? Instead of an integrated tool like Cargo, it's
a composition of independent tools (to some extent) where the responsibilities
are roughly distributed like this:

- autoconf: detect build dependencies
- automake: build and tests scaffolding
- make: turn sources into targets
- rustc: turn Rust code into binaries
- rst2man: turn RST files into manual pages
- varnishtest: run individual test cases
- kcov: collect code coverage from the test suite

Two more interesting points, the autotools can create dist archives, source
archives that can be redistributed independently of the autotools. And none of
the tools are responsible for package or dependencies management. As a Fedora
contributor, RPM is my go-to tool. I will probably add built-in RPM support to
this project at some point, it's easy. Regarding dependencies, they were all
installed via RPM except Varnish for which all releases are installed manually
on my system. And this project is meant to be written with no external crate,
even one that could make parsing simpler, for educational purposes.

This brings me to the next topic: external crates. I don't like how languages
like Go or Rust manage dependencies. I don't like it, it doesn't mean that I
don't understand at least some of the constraints that make it necessary (at
least for Rust). The Fedora project has four principles all starting with an
F and the interesting one here is First. We should aim at leaning towards
latest versions of upstream projects and ideally be the first to get there.
There is also a strong anti-bundling policy that favors shared libraries and
discourages static linking. Tool chains like Rust's and Go's were granted
exceptions because the tooling doesn't make it easy to get rid of static
linking and bringing any Rust project in the distribution would throw a huge
burden at the package maintainers.

Late in the game enough people from the Fedora project took notice of Rust,
formed a Special Interest Group (Rust SIG) and started bending the Cargo
roadmap to help it fit in the ecosystem. Thanks to them and Rust's commitment
to stability it only takes a week for the latest release to land on my system.
So now I assume that Cargo is capable of delegating dependency management (in
our case to RPM) and merely detect dependencies, which means it can satisfy
the offline build requirement of Fedora packages. There are probably other
things that needed to be adjusted but this is still not enough to get me
interested in Cargo. I should also mention that I also didn't follow closely
the Rust SIG's involvment in Rust and Cargo.

Bundling dependencies can hinder Fedora's march forward. If different projects
need different versions of a same dependency we have two choices. Either we
help move those projects forward or we package older versions of dependencies
in what we call compat packages. Ideally libraries (or even programs) don't
break their API or ABI in the case of dynamic linking, and let you know when
that happens (for example ELF shared objects may bump their soname or maintain
versioned symbols). In Rust's case, we are only dependent on the API of Rust
crates because of static linking, and we supposedly never need to rebuild a
package (except for security updates) unless we update the package itself.
Rust doesn't have a stable ABI, and maintaining one would likely prevent the
huge progress we witness every six weeks on new releases. The only thing that
may really get in the way of avoiding compat packages is the ``Cargo.lock``
file that is recommeded to check in in order to get a stable snapshot of the
dependencies at any time. This is a difficult trade off overall.

The main reason why I prefer ``make`` over <insert language-specific build
system here> is the level of abstraction. You build target from sources
using the commands of your choice, targets may in turn be sources to other
targets and so on. ``make`` doesn't care whether your building a C or Rust
project (but the autotools do to some large extent!) so mixing both is a no
brainer. Sadly ``automake`` makes definite assumptions on how a program
should be built that is plain incompatible with how ``rustc`` works. It should
be possible to add Rust support to ``libtool`` but I'll put my blinders on and
pretend I didn't even entertain the idea. But ``automake`` comes with one more
interesting feature: a test driver.

So what? Cargo does too. But once again (stale comment alert) last time I
checked it was only about unit testing. Because I can't see a clear definition
of what a unit is in the wild (hint: languages with different paradigms) I'm
talking about "code testing" instead. With cargo (or rustc? I can't remember)
you can test your Rust code with Rust code. I'm OK with that only if the test
code is strictly using public APIs, of a library. ``vclpp`` is a program, and
in order to really test it it should be launched by the test suite. The best
abstraction for that is the shell.

This is the same abstraction used by ``make``: it takes care of solving the
dependencies between sources and targets, and shells out the commands that
actually lead from the former to the latter. In this case, the test drivers
delegates the test execution to ``varnishtest``: the test framework from
Varnish Cache. Most of the test cases are glorified make targets in the sense
that they run shell commands to run a scenario and check the results. In the
initial test suite, only one case truly uses ``varnishtest`` to load VCL code
in Varnish and confirm that preprocessing all went well as expected. While
that may seem overkill, the test reports are rather nice and already collected
by ``automake``, so that's another reason why I'm using it.

Another advantage of the shell is the ability to embed PVCL code directly in
the shell code via a here-document. To Rust's credit, multi-line strings are
so nothing-special that this doesn't even count as an argument.

One more thing then, ``kcov`` was mentioned. This wasn't trivial to integrate
transparently in the test suite (because I insisted on transparency) but after
figuring how it works I could measure a whopping 90% coverage (which isn't
even impressive for such a tiny code base). I found it so convenient that I
submitted a package for Fedora.

So what was the point of that lengthy rant already? Oh yes, the cargo cult.
This isn't exactly a rant, rather a praise. I've been itching to get a real
project in Rust for years and finally it has come. This is a tiny project but
aren't they the best when it comes to learning? Small enough to wrap one's
head around but actually useful. I don't like Cargo, this is my problem, but
Cargo has also been my Linux distribution of choice's problem for a while too.
And Rust doesn't even force me to use it if I don't want to? Yep, definitely
not complaining here.

-- Dridi

PS. With Rust 1.19 I can write unsafe Rust without an unsafe block and only
using the ``std`` crate. Fearless concurrency? I think not, but there is a
catch ;-)
