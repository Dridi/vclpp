VCL preprocessor
================

This is ``vclpp``, a working preprocessor for VCL in order to experiment
alternative syntaxes for the Varnish Configuration Language. This is also an
experiment in Rust, a programming language with the reputation of being nice
for writing parsers.

History
-------

VCL is a domain-specific language that has two facets: configuration in the
form of backend and other declarations (mainly augmented with a ``vcl_init``
subroutine) and a policy that runs HTTP transactions via the bulk of the
available subroutines. Arguably since Varnish 4 the policy facet is split in
two with the segregation of client and backend transactions.

In the configuration part, items are declared with the following syntax::

  type name {
      # some kind of definition
  }

For example::

  backend www {
      .host = "localhost";
      .port = 8080;
  }

Up until Varnish 3 such items included directors, logical clusters of
backends::

  director www fallback {
      { .backend = www1; }
      { .backend = www2; }
      { .backend = www3; }
  }

In Varnish 4 they disappeared in favour of VMOD objects, and most of the
existing directors ended up in a VMOD called (with much originality)
``directors``::

  import directors;

  sub vcl_init {
      new www = directors.fallback();
      www.add_backend(www1);
      www.add_backend(www2);
      www.add_backend(www3);
  }

The major downside is one of the big syntax breakages introduced by Varnish 4
and the programmatic nature of this configuration put off at least some people
who don't necessarily appreciate the benefits. By losing the ability to use
directors transparently like backends, we gained two interesting things. We
are no longer constrained to whatever director Varnish comes up with, and some
directors are fit for composition. You can for example have two round-robin
clusters inside a fallback cluster and thus implement an active-passive model.

The idea of a VCL preprocessor was born on those premises.

Goals
-----

The goal of ``vclpp`` is to explore alternative syntaxes for VCL, and help
make configuration more declarative and leave the programmatic aspects to the
cache policy.

The first/main idea is to turn VMOD objects into items similar to backends and
probes and make them less special while at the same time ensuring determinism
of the configuration phase.

Consider this code::

  sub vcl_init {
      if (some environment) {
          new www = directors.fallback();
          ...
      }

      if (some other environment) {
          new www = directors.round_robin();
          ...
      }
  }

The ``www`` object may never be initialized, and of course this could be
solved by turning an ``if`` series into an ``if-elsif-else`` construct but
then again what if one condition spuriously fails? We can't really fix this
in the VCL compiler without introducing some kind of static analysis, which
is probably an unrealistic goal.

The only sensible reason to have conditionals in ``vcl_init`` is probably to
bail out if anything goes wrong::

  sub vcl_init {
      [...]
      if (something not OK) {
          return (fail);
      }
  }

An alternative is to separate the cache policy from the environment-specific
configuration::

  vcl 4.0;

  import directors;

  include "environment.vcl";
  include "policy.vcl";

This way, all you need to do is ship the correct ``environment.vcl`` to the
correct environments (dev, test, prod...) and keep a branch-less ``vcl_init``.
To go even further, let's change the syntax to get rid of the ``new`` keyword
and pretend it's all declarative::

  directors.fallback www {
      .add_backend(www1);
      .add_backend(www2);
      .add_backend(www3);
  }

This is the first "alternative syntax" explored by ``vclpp``, documented in
man pages. It turns this kind of declarative block into a ``vcl_init`` block
with the ``new`` syntax that Varnish expects. In addition it can call methods
once the object is constructed while still retaining the declarative style.

It works because you can have more than one ``vcl_*`` subroutine at a time,
the result being the concatenation of all subroutine into a single one.

Non goals
---------

The good news is that ``vclpp`` doesn't need to know anything about the VMODs
involved in the process. The grammar itself is enough to produce valid VCL and
``varnishd`` will ultimately decide whether the VCL is correct. So ``vclpp``
doesn't want to be a comprehensive VCL parser and knows just enough to turn
alternative syntaxes into equivalent VCL.

The implementation works in a single pass, and is not suitable for all cases,
like ``include`` statements inside blocks of code. Known limitations are
documented in the manuals and will at best be worked around.

It might be necessary to keep track of some kind of state across executions of
``vclpp`` to make some syntaxes work on a file and its includes. This is too
bothersome to deal with (the goal is only to explore the syntax space of VCL)
and VCL labels and the ability to switch to labels may offer a better-suited
compromise.

This is not a C-like preprocessor based on macros substitution or expansion.
Templating tools can already be used in the delivery area and are probably
already good at that. Moving to a declarative syntax may even prove easier to
rely on such tooling (feedback welcome).

How to use it
-------------

The command-line interface for ``vclpp`` is very simple and bare-bones. There
are no options, only up to two arguments for the input PVCL file and the
output VCL file. By default they fall back to standard input and output. See
the manual for more details, this may evolve in the future.

For example, a systemd integration can be as simple as::

  ExecStartPre=/usr/bin/vclpp /etc/varnish/main.pvcl /etc/varnish/main.vcl

If you are using labels, you can add as many ``ExecStartPre`` options as you
need to process all your PVCL files. This is true for includes too, but some
limitations are documented in the ``pvcl(7)`` manual.

Contributing
------------

The simplest way to contribute is reporting a problem by opening a Github
issue_.

.. _issue: https://github.com/dridi/vclpp/issues/new

Even if you are not planning to use ``vclpp``, you can try it with regular VCL
code and check whether the output is identical to the original file. It should
be, otherwise it's a bug. In that case, please try to reproduce the bug with
minimal VCL and open a Github issue.

Whether you are trying ``vclpp`` with VCL or PVCL, if the program crashes with
a message looking like 'internal error: entered unreachable code' please also
open a Github issue. This should highlight an overlook in the tokenizer or the
preprocessor.

If you have an idea of how to improve VCL that could be tested via ``vclpp``,
you are also welcome to open a Github issue and spawn a discussion.

Finally, if you are a Rust enthusiast and have a clear idea of how things
could be better implemented TheRightWay(tm) suggestions via a Github issue are
also most welcome. ``vclpp`` is not your average Rust project, the reasons are
detailed in the installation notes.
