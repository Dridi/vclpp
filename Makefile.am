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

doc_DATA = LICENSE

bin_SCRIPTS = src/vclpp
noinst_SCRIPTS = src/vcltok

src/vclpp: src/vclpp.rs src/cli.rs src/tok.rs
	@mkdir -p src/
	$(RUSTC) -g -o $@ $<

src/vcltok: src/vcltok.rs src/cli.rs src/tok.rs
	@mkdir -p src/
	$(RUSTC) -g -o $@ $<

AM_TESTS_ENVIRONMENT = \
	KCOV="$(KCOV)" \
	PATH="$(VARNISH_TEST_PATH):$(VCLPP_PATH):$(PATH)" \
	LD_LIBRARY_PATH="$(VARNISH_LIBRARY_PATH)"
TEST_EXTENSIONS = .vtc
VTC_LOG_COMPILER = varnishtest -v

TESTS = \
	tests/00-tokenization.vtc \
	tests/01-hello-world.vtc \
	tests/02-empty-object.vtc \
	tests/03-complex-object.vtc \
	tests/04-command-line.vtc

check: all

kcov:
	rm -rf kcov/
	$(MAKE) $(AM_MAKEFLAGS) check
	$(KCOV) --merge kcov/ kcov/vtc/*/*/
	@echo "results: $(abs_builddir)/kcov/index.html"

.PHONY: kcov

clean-local:
	rm -rf kcov/

CLEANFILES = \
	src/vclpp \
	src/vcltok

EXTRA_DIST = \
	LICENSE \
	src/vclpp.rs \
	src/vcltok.rs \
	src/cli.rs \
	src/tok.rs \
	$(TESTS)
