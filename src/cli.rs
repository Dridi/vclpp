/*-
 * vclpp
 * Copyright (C) 2017  Dridi Boukelmoune <dridi.boukelmoune@gmail.com>
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <http://www.gnu.org/licenses/>.
 */

use std::cmp::Ordering::Equal;
use std::env;
use std::fmt::Display;
use std::fs::File;
use std::io::BufWriter;
use std::io::Read;
use std::io::Result;
use std::io::Stdout;
use std::io::Write;
use std::io::stdin;
use std::io::stdout;
use std::process::exit;

use self::Output::*;

pub enum Output {
    Arg(BufWriter<File>),
    Def(BufWriter<Stdout>),
}

impl Output {
    fn arg(f: File) -> Output { Arg(BufWriter::new(f)) }
    fn def() -> Output { Def(BufWriter::new(stdout())) }
}

impl Write for Output {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            &mut Arg(ref mut bw) => bw.write(buf),
            &mut Def(ref mut bw) => bw.write(buf),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            &mut Arg(ref mut bw) => bw.flush(),
            &mut Def(ref mut bw) => bw.flush(),
        }
    }
}

fn eprint_usage(arg0: String) -> ! {
    eprintln!("VCL preprocessor

Usage:
    {} [PVCL [VCL]]

Documentation available locally via `man vclpp`", arg0);
    exit(1);
} // unreachable

pub fn parse_args() -> Result<(String, Output)> {
    let mut args = env::args();

    let arg0 = args.next().unwrap();

    if args.len() > 2 {
        eprint_usage(arg0);
    }

    let mut src = String::new();

    match args.next() {
        Some(path) => match path.cmp(&"-".to_string()) {
            Equal => stdin().read_to_string(&mut src)?,
            _ => File::open(path)?.read_to_string(&mut src)?,
        },
        None => stdin().read_to_string(&mut src)?,
    };

    let out = match args.next() {
        Some(path) => match path.cmp(&"-".to_string()) {
            Equal => Output::def(),
            _ => Output::arg(File::create(path)?),
        },
        None => Output::def(),
    };

    Ok((src, out))
}

pub fn fail<T: Display>(s: T) -> ! {
    eprintln!("Error: {}", s);
    exit(1);
} // unreachable
