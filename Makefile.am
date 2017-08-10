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

ACLOCAL_AMFLAGS = -I .

DISTCHECK_CONFIGURE_FLAGS = --with-tests

doc_DATA = LICENSE

bin_SCRIPTS = src/vclpp
noinst_SCRIPTS = src/vcltok

vclpp_SRC = \
	src/vclpp.rs \
	src/cli.rs \
	src/tok.rs

vcltok_SRC = \
	src/vcltok.rs \
	src/cli.rs \
	src/tok.rs

src/vclpp: $(vclpp_SRC)
	@mkdir -p src/
	$(RUSTC) $(RUSTC_FLAGS) -o $@ $(srcdir)/src/vclpp.rs

src/vcltok: $(vcltok_SRC)
	@mkdir -p src/
	$(RUSTC) $(RUSTC_FLAGS) -o $@ $(srcdir)/src/vcltok.rs

dist_man_MANS = \
	man/pvcl.7 \
	man/vclpp.1

SUFFIXES = .1.rst .7.rst .1 .7

.1.rst.1:
	$(RST2MAN) --strict --strip-comments $< $@

.7.rst.7:
	$(RST2MAN) --strict --strip-comments $< $@

AM_TESTS_ENVIRONMENT = \
	KCOV="$(KCOV)" \
	PATH="$(VCLPP_PATH):$(VARNISH_TEST_PATH):$(PATH)" \
	LD_LIBRARY_PATH="$(VARNISH_LIBRARY_PATH)"
TEST_EXTENSIONS = .vtc
VTC_LOG_COMPILER = varnishtest -v

VTC_TESTS = \
	vtc/00-tokenization.vtc \
	vtc/01-hello-world.vtc \
	vtc/02-empty-object.vtc \
	vtc/03-complex-object.vtc \
	vtc/04-command-line.vtc \
	vtc/05-syntax-errors.vtc


if WITH_TESTS
TESTS = $(VTC_TESTS)

check: all

kcov:
	rm -rf kcov/
	$(MAKE) $(AM_MAKEFLAGS) clean
	$(MAKE) $(AM_MAKEFLAGS) RUSTC_FLAGS=-g check
	$(KCOV) --merge kcov/ kcov/vtc/*/*/
	@echo "results: $(abs_builddir)/kcov/index.html"

.PHONY: kcov

clean-local:
	rm -rf kcov/
endif

CLEANFILES = \
	src/vclpp \
	src/vcltok

EXTRA_DIST = \
	LICENSE \
	README.rst \
	src/vclpp.rs \
	src/vcltok.rs \
	src/cli.rs \
	src/tok.rs \
	vclpp.spec \
	$(VTC_TESTS)
