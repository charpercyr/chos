
#[macro_export]
macro_rules! elf_table {
    ($lif:lifetime, $ty:ident, $entries:ident, $entry:ident, $iter:ident, $inner_iter:ty) => {
        impl<$lif> $ty<$lif> {
            pub fn get(& $lif self, idx: usize) -> $entry<$lif> {
                $entry::new(&self.$entries[idx])
            }

            pub fn iter(&self) -> $iter<$lif> {
                $iter {
                    iter: self.$entries.iter(),
                }
            }
        }

        #[derive(Clone)]
        pub struct $iter<$lif> {
            iter: $inner_iter,
        }

        impl<$lif> Iterator for $iter<$lif> {
            type Item = $entry<$lif>;
            fn next(&mut self) -> Option<Self::Item> {
                self.iter.next().map(|e| $entry::new(e))
            }
        }

        impl<$lif> IntoIterator for $ty<$lif> {
            type Item = $entry<$lif>;
            type IntoIter = $iter<$lif>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }

        impl<$lif> IntoIterator for &$ty<$lif> {
            type Item = $entry<$lif>;
            type IntoIter = $iter<$lif>;
            fn into_iter(self) -> Self::IntoIter {
                self.iter()
            }
        }
    };
}
