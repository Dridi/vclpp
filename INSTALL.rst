Installation instructions
=========================

Unlike usual Rust code bases, ``vclpp`` uses the GNU build system in order to
integrate well with Varnish and especially ``varnishtest`` to run the test
suite.

Dependencies:

- Rust >= 1.19
- Varnish >= 5.1.3
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
options you can run ``./configure --help``.

Once installed, ``vclpp`` has no runtime dependencies.

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
