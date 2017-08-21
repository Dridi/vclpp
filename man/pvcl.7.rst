.. vclpp
.. Copyright (C) 2017  Dridi Boukelmoune <dridi.boukelmoune@gmail.com>
..
.. This program is free software: you can redistribute it and/or modify
.. it under the terms of the GNU General Public License as published by
.. the Free Software Foundation, either version 3 of the License, or
.. (at your option) any later version.
..
.. This program is distributed in the hope that it will be useful,
.. but WITHOUT ANY WARRANTY; without even the implied warranty of
.. MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
.. GNU General Public License for more details.
..
.. You should have received a copy of the GNU General Public License
.. along with this program.  If not, see <http://www.gnu.org/licenses/>.

====
pvcl
====

--------------
Pre-VCL syntax
--------------

:Manual section: 7

DESCRIPTION
===========

``vclpp`` isn't meant to be a preprocessor to VCL like ``cpp`` is one to C.
The goal is not to have macro substitution for regular VCL but rather explore
alternative syntaxes. The PVCL language is a superset of VCL so regular VCL
code is left untouched, in addition it offers new language constructs. The
alternative syntaxes can be combined, they are orthogonal.

Declarative objects (since vclpp 0.1)
-------------------------------------

This is an alternative to the ``new`` keyword needed to instantiate objects
in ``vcl_init``. The alternative syntax looks like a regular VCL declaration
and can even be used to call methods.

For example, stacking directors::

  vcl 4.0;

  import directors;

  probe default { }

  backend www_fr { ... }
  backend www_de { ... }
  backend www_us { ... }
  backend www_ca { ... }

  directors.round_robin www_eu {
      .add_backend(www_fr);
      .add_backend(www_de);
  }

  directors.round_robin www_na {
      .add_backend(www_us);
      .add_backend(www_ca);
  }

  directors.fallback www {
      .add_backend(www_eu.backend());
      .add_backend(www_na.backend());
  }

  sub vcl_recv {
      set req.backend_hint = www.backend();
  }

Declarative objects borrow the syntax of ``backend`` and ``probe`` and add the
possibility to instantiate and initialize objects using a familiar syntax as
an alternative to the programmatic approach in ``vcl_init``.

The general syntax is::

  vmod.constructor object {
      .attribute = value;
      .method(parameters);
  }

All attributes must be declared before method calls, they must match arguments
to the constructor. So the VMOD descriptor needs to include the names of all
parameters at least for a constructor. Neither attributes nor method calls are
mandatory for VMOD objects that don't need them::

  vmod.constructor no_args { }

Consider another example::

  querystring.filter qf {
      .match = name;
      .sort = true;
      .add_string("_"); # a timestamp used to bypass caches
      .add_glob("utm_*"); # google analytics parameters
      .add_regex("sess[0-9]+"); # anti-CSRF token
  }

The comments inside a declarative object are lost, and the PVCL code above
would be translated in VCL as::

  sub vcl_init {
      new qf = querystring.filter(
          match = name,
          sort = true);
      qf.add_string("_");
      qf.add_glob("utm_*");
      qf.add_regex("sess[0-9]+");
  }

``vclpp`` uses tabulations for indentation.

Request authority (since vclpp 0.1)
-----------------------------------

This is an alternative to ``req.http.host``. The host header was introduced in
HTTP/1.1 to enable virtual hosting, that is to say the use of multiple domains
for a single address. In HTTP/2, pseudo headers were introduced for non-header
fields (method, URI, status) and the host header was promoted at the same time
to an ``:authority`` pseudo-header.

The host header is an important one in Varnish (or for that matter HTTP in
general) but doesn't usually get as much credit as the URL. However, it is so
important that it's part of the default hash. Here is the default ``vcl_hash``
sub-routine with the proposed syntax::

  sub vcl_hash {
      hash_data(req.url);
      if (req.authority) {
          hash_data(req.authority);
      } else {
          hash_data(server.ip);
      }
      return (lookup);
  }

The authority is now as important as the URL and of course ``bereq.authority``
is supported too. Interesting trivia, HTTP/2 introduced a ``:path`` pseudo
header and Varnish already dissects client URLs to extract only the path in
``req.url``. Does this call for a ``req.path`` alternative syntax too?

VMOD aliases (since vclpp 0.1)
------------------------------

One of the grievances that sparkled from the introduction of VCL 4.0 was the
verbosity of the new directors. Depending on their nomenclature, people could
end up with much less readable code::

  sub vcl_init {
      new <long-name> = <long-name>.<long-name>(<lots-of-parameters>);
      ...
  }

Named parameters introduced by Varnish 4.1 could help, a lot, but they would
only solve part of a problem. The declarative objects syntax already mitigates
this problem greatly, but it only helps with VMOD objects. If for some reason
you are not happy with a VMOD name, you can always pick one that suits you
better::

  import directors as lb;

  sub vcl_init {
    new example_com_wordpress_cluster_eu = lb.round_robin();
    ...
  }

Most VMODs have short names or are confined to ``vcl_init`` where declarative
objects would usually do a better job at keeping the code concise and killing
needless duplication, so this syntax is on the cosmetic side of the fence.

LIMITATIONS
===========

The first big limitation is that ``vclpp`` can only process UTF-8 files.

VCL already has some degree of preprocessing in place. First, it can be
considered a preprocessor for C since it translates to C code. And second,
there is the expansion of ``include`` statements. An included VCL file can
be hard to use with ``vclpp``.

Consider the following example::

  vcl 4.0;

  import std;
  import directors;

  include "environment.vcl"
  include "policy.vcl"

The main file along with the two included files can probably be safely
preprocessed by ``vclpp`` (assuming there aren't any more nested includes)
but some files would be relevant for some features, and other features may
break with such a setup. ``environment.vcl`` is typically where you would find
backend and director definitions and ``policy.vcl`` would contain transaction
sub-routines instead.

Now consider this case::

  if (req.http.some-header == "some-value") {
      include "some-policy.vcl";
  }

This VCL snippet is not valid as a whole VCL but could well be included and
be valid as part of the surrounding VCL. So it hard to guess, though not
impossible, whether this code starts at the root of a VCL file (as in not
inside a block) and the same goes for ``some-policy.vcl``.

Even if it is possible to infer that ``if`` needs to be nested at least in a
subroutine and therefore that it couldn't be at the root, ``vclpp`` does a
single pass and could be mislead before reaching this statement. Of course at
this point it could fail gracefully (but would have already output some code)
but this is not the case yet.

In summary, ``vclpp`` doesn't expand includes and leaves them as-is but also
has no way of knowing yet the level of nesting of included fragments.

COPYRIGHT
=========

This document is licensed under the same license as ``vclpp`` itself, see
LICENSE for details.

SEE ALSO
========

**vcl**\(7),
**vclpp**\(1)
