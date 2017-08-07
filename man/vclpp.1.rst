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

=====
vclpp
=====

----------------
VCL preprocessor
----------------

:Manual section: 1

SYNOPSYS
========

**vclpp** [*PVCL* [*VCL*]]

DESCRIPTION
===========

Reads a "pre-VCL" file and turns it into a regular VCL file that can be loaded
by ``varnishd``. If *PVCL* is a regular VCL file, the output is identical. If
*PVCL* or *VCL* is omitted ``-``, it is read or written respectively from the
standard input or to the standard output.

COPYRIGHT
=========

This document is licensed under the same license as ``vclpp`` itself, see
LICENSE for details.

SEE ALSO
========

**pvcl**\(7),
**vcl**\(7)
