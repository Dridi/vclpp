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
#
# rust.m4 - Macros for the Rust programming language
# serial 1

# RUST_PREREQ(MINIMUM-VERSION)
# ----------------------------
AC_DEFUN([RUST_PREREQ], [
	AC_REQUIRE([AC_PROG_AWK])

	AC_CHECK_PROGS([RUSTC], [rustc])
	AS_IF([test -z "$RUSTC"], [AC_MSG_ERROR([Rust compiler required.])])

	RUSTC_VERSION=$("$RUSTC" --version | awk '{print $[]2}')
	AC_MSG_CHECKING([for rust $1+])
	RUSTC_ENOUGH=yes
	AS_VERSION_COMPARE([$RUSTC_VERSION], [$1], [RUSTC_ENOUGH=no])
	AC_MSG_RESULT([$RUSTC_VERSION])

	AS_IF([test "$RUSTC_ENOUGH" = no], [
		AC_MSG_ERROR([Rust version $1 or higher is required.])
	])
])
