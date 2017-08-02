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

bin_SCRIPTS = src/vclpp

src/vclpp: src/vclpp.rs src/tok.rs
	@mkdir -p src/
	$(RUSTC) -g -o $@ $<

AM_TESTS_ENVIRONMENT = \
	PATH="$(VARNISH_TEST_PATH):$(VCLPP_PATH):$(PATH)" \
	LD_LIBRARY_PATH="$(VARNISH_LIBRARY_PATH)"
TEST_EXTENSIONS = .vtc
VTC_LOG_COMPILER = varnishtest -v

TESTS = \
	tests/01-hello-world.vtc \
	tests/02-empty-object.vtc

doc_DATA = LICENSE

CLEANFILES = src/vclpp
EXTRA_DIST = \
	LICENSE \
	src/vclpp.rs \
	src/tok.rs \
	$(TESTS)
