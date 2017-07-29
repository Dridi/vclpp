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
AC_CONFIG_SRCDIR([vclpp.rs])

AM_INIT_AUTOMAKE([1.12 -Wall -Werror foreign parallel-tests])
AC_CHECK_PROGS([RUSTC], [rustc])
VARNISH_PREREQ([5.1.2])
AC_CONFIG_FILES([Makefile])
AC_OUTPUT

AS_ECHO("
	==== $PACKAGE_STRING ====

	varnish:      $VARNISH_VERSION
	prefix:       $prefix
")
