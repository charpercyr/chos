
use core::str::Split;
pub struct CmdLine<'a> {
    iter: Split<'a, char>,
}

impl<'a> Iterator for CmdLine<'a> {
    type Item = (&'a str, Option<&'a str>);

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next().map(|kv| {
            match kv.find('=') {
                Some(idx) => (&kv[..idx], Some(&kv[idx+1..])),
                None => (kv, None),
            }
        })
    }
}

pub fn iter_cmdline(cmdline: &str) -> CmdLine<'_> {
    CmdLine {
        iter: cmdline.split(' '),
    }
}
