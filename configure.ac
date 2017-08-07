# vclpp
# Copyright (C) 2017  Dridi Boukelmoune <dridi.boukelmoune@gmail.com>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License
# along with this program.  If not, see <http://www.gnu.org/licenses/>.

AC_PREREQ([2.68])
AC_INIT([vclpp],
	[0.1],
	[https://github.com/dridi/vclpp/issues],
	[vclpp],
	[https://github.com/dridi/vclpp])
AC_COPYRIGHT([Copyright (C) 2017  Dridi Boukelmoune])
AC_CONFIG_MACRO_DIR([m4])
AC_CONFIG_AUX_DIR([build-aux])
AC_CONFIG_SRCDIR([src/vclpp.rs])

# Don't inherit Varnish's prefix by default
AS_IF([test "$prefix" = NONE], [prefix=/usr/local])

AM_INIT_AUTOMAKE([1.12 -Wall -Werror foreign parallel-tests])
RUST_PREREQ([1.19])
VARNISH_PREREQ([5.1.3])

AC_ARG_VAR([RST2MAN], [rst2man command (optional)])
AC_CHECK_PROGS([RST2MAN], [rst2man rst2man.py])
AS_IF([test -z "$RST2MAN"], [RST2MAN=":"])

AC_ARG_VAR([KCOV], [kcov command (optional)])
AC_ARG_WITH([kcov],
	[AS_HELP_STRING([--with-kcov], [use kcov (default is no)])],
	[use_kcov=$withval],
	[use_kcov=no])

AS_IF([test "$use_kcov" != no], [AC_CHECK_PROGS([KCOV], [kcov])])

AC_SUBST([VCLPP_PATH], ['$(abs_builddir)'])

AC_CONFIG_FILES([
	Makefile
	kcov-wrapper.sh
])
AC_OUTPUT

ln -f -s kcov-wrapper.sh vclpp
ln -f -s kcov-wrapper.sh vcltok
chmod +x kcov-wrapper.sh

AS_ECHO("
	==== $PACKAGE_STRING ====

	varnish:      $VARNISH_VERSION
	prefix:       $prefix
")
