use cake::log::info;

use crate::{
    align,
    paging::{
        Large, Medium, PhysAddr, PrimitiveSize, Small, VirtAddr,
        primitives::{
            Address, AnyPrimitive, FrameClass, MemoryFragment, PageClass, PrimitiveClass,
        },
    },
    test_println,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GreedyFragmentMapper<Class: PrimitiveClass> {
    base_prim: AnyPrimitive<Class>,
    len: u64,
    remain: u64,
}

impl<Class> GreedyFragmentMapper<Class>
where
    Class: PrimitiveClass,
{
    pub fn new(base: Class::Addr, len: u64) -> Self {
        Self {
            base_prim: Self::next_prim(base, len),
            len,
            remain: len,
        }
    }

    fn next_prim(base: Class::Addr, len: u64) -> AnyPrimitive<Class> {
        let aligned_base = align!(down, base.as_u64(), Small::SIZE);

        if aligned_base % Large::SIZE == 0 && len >= Large::SIZE {
            return AnyPrimitive::Large(
                Class::Fragment::from_start_address(Class::Addr::new(aligned_base)).unwrap(),
            );
        }

        if aligned_base % Medium::SIZE == 0 && len >= Medium::SIZE {
            return AnyPrimitive::Medium(
                Class::Fragment::from_start_address(Class::Addr::new(aligned_base)).unwrap(),
            );
        }

        AnyPrimitive::Small(
            Class::Fragment::from_start_address(Class::Addr::new(aligned_base)).unwrap(),
        )
    }

    /// Returns the next memory primitive without advancing the iterator.
    pub fn peek_next_fragment(&self) -> Option<AnyPrimitive<Class>> {
        if self.remain == 0 {
            return None;
        }
        Some(self.base_prim)
    }

    /// Returns the next memory primitive and advances the iterator.
    pub fn next_fragment(&mut self) -> Option<AnyPrimitive<Class>> {
        if self.remain == 0 {
            return None;
        }

        let next_prim = self.base_prim;
        let next_prim_size = next_prim.size();
        let next_prim_end = next_prim.start_address() + next_prim_size;

        self.remain = self.remain.saturating_sub(next_prim_size);
        self.base_prim = Self::next_prim(next_prim_end, self.remain);
        Some(next_prim)
    }

    pub fn try_take_same<C: PrimitiveClass>(
        &mut self,
        prim: AnyPrimitive<C>,
    ) -> Option<AnyPrimitive<Class>> {
        if self.remain == 0 {
            return None;
        }

        let start = self.base_prim;
        let start_addr = self.base_prim.start_address();

        match prim {
            AnyPrimitive::Small(_) => {
                let start = start.downsize_as();
                self.remain = self.remain.saturating_sub(Small::SIZE);
                self.base_prim = Self::next_prim(start_addr + Small::SIZE, self.remain);
                Some(AnyPrimitive::Small(start))
            }
            AnyPrimitive::Medium(_) => {
                // Don't allocate a huge amount of extra memory if the remaining size could fit in a small page.
                if self.remain <= Small::SIZE {
                    return None;
                }
                let start = start.downsize_as();
                self.remain = self.remain.saturating_sub(Medium::SIZE);
                self.base_prim = Self::next_prim(start_addr + Medium::SIZE, self.remain);
                Some(AnyPrimitive::Medium(start))
            }
            AnyPrimitive::Large(_) => {
                // Don't allocate a huge amount of extra memory if the remaining size could fit in a small or medium page.
                if self.remain <= Medium::SIZE {
                    return None;
                }
                let start = start.downsize_as();
                self.remain = self.remain.saturating_sub(Large::SIZE);
                self.base_prim = Self::next_prim(start_addr + Large::SIZE, self.remain);
                Some(AnyPrimitive::Large(start))
            }
        }
    }
}

impl<Class> Iterator for GreedyFragmentMapper<Class>
where
    Class: PrimitiveClass,
{
    type Item = AnyPrimitive<Class>;

    fn next(&mut self) -> Option<Self::Item> {
        self.next_fragment()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct JointFragmentMapper {
    page_mapper: GreedyFragmentMapper<PageClass>,
    frame_mapper: GreedyFragmentMapper<FrameClass>,
}

impl JointFragmentMapper {
    pub fn new(virt_base: VirtAddr, phys_base: PhysAddr, len: u64) -> Self {
        Self {
            page_mapper: GreedyFragmentMapper::new(virt_base, len),
            frame_mapper: GreedyFragmentMapper::new(phys_base, len),
        }
    }
}

impl Iterator for JointFragmentMapper {
    type Item = (AnyPrimitive<PageClass>, AnyPrimitive<FrameClass>);

    fn next(&mut self) -> Option<Self::Item> {
        let page_frag = self.page_mapper.peek_next_fragment()?;
        let frame_frag = self.frame_mapper.peek_next_fragment()?;
        match (page_frag, frame_frag) {
            (AnyPrimitive::Small(_), AnyPrimitive::Small(_))
            | (AnyPrimitive::Medium(_), AnyPrimitive::Medium(_))
            | (AnyPrimitive::Large(_), AnyPrimitive::Large(_)) => {
                return Some((
                    self.page_mapper.next().unwrap(),
                    self.frame_mapper.next().unwrap(),
                ));
            }
            (page_frag, frame_frag) => {
                if page_frag.size() < frame_frag.size() {
                    let page_frag = self.page_mapper.next().unwrap();
                    let sliced_frame_frag = self.frame_mapper.try_take_same(page_frag).unwrap();
                    test_println!(
                        "page smaller: page_frag {:#?} is smaller than frame_frag {:#?}, taking page_frag and matching frame_frag {:#?}",
                        page_frag,
                        frame_frag,
                        sliced_frame_frag
                    );
                    return Some((page_frag, sliced_frame_frag));
                } else {
                    let frame_frag = self.frame_mapper.next().unwrap();
                    let sliced_page_frag = self.page_mapper.try_take_same(frame_frag).unwrap();
                    test_println!(
                        "frame smaller: frame_frag {:#?} is smaller than page_frag {:#?}, taking frame_frag and matching page_frag {:#?}",
                        frame_frag,
                        page_frag,
                        sliced_page_frag
                    );
                    return Some((sliced_page_frag, frame_frag));
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use crate::paging::{
        Address, Frame, Large, Medium, Page, PhysAddr, PrimitiveSize, Small, VirtAddr,
        fragment::{GreedyFragmentMapper, JointFragmentMapper},
        primitives::{AnyPage, AnyPrimitive, FrameClass, PageClass},
    };

    fn test_create_base_prim() {
        use crate::paging::primitives::{AnyPrimitive, PrimitiveClass, Small};

        let test = |base: u64, len: u64, expected_base: AnyPrimitive<FrameClass>| {
            let prim = GreedyFragmentMapper::<FrameClass>::next_prim(PhysAddr::new(base), len);
            assert_eq!(prim, expected_base);
        };

        test(
            Small::SIZE,
            Small::SIZE,
            AnyPrimitive::Small(
                Frame::<Small>::from_start_address(PhysAddr::new(Small::SIZE)).unwrap(),
            ),
        );
        test(
            Medium::SIZE,
            Medium::SIZE,
            AnyPrimitive::Medium(
                Frame::<Medium>::from_start_address(PhysAddr::new(Medium::SIZE)).unwrap(),
            ),
        );
        test(
            Large::SIZE,
            Large::SIZE,
            AnyPrimitive::Large(
                Frame::<Large>::from_start_address(PhysAddr::new(Large::SIZE)).unwrap(),
            ),
        );

        test(
            Small::SIZE + 1,
            Small::SIZE,
            AnyPrimitive::Small(
                Frame::<Small>::from_start_address(PhysAddr::new(Small::SIZE)).unwrap(),
            ),
        );
        test(
            Medium::SIZE + 1,
            Medium::SIZE,
            AnyPrimitive::Medium(
                Frame::<Medium>::from_start_address(PhysAddr::new(Medium::SIZE)).unwrap(),
            ),
        );
        test(
            Large::SIZE + 1,
            Large::SIZE,
            AnyPrimitive::Large(
                Frame::<Large>::from_start_address(PhysAddr::new(Large::SIZE)).unwrap(),
            ),
        );
    }

    fn new(vaddr: u64, len: u64) -> GreedyFragmentMapper<PageClass> {
        GreedyFragmentMapper::<PageClass>::new(VirtAddr::new(vaddr), len)
    }

    #[test]
    fn test_greedy_next_fragment() {
        let mut mapper = new(3, 3 * Large::SIZE + Medium::SIZE);

        assert_eq!(
            mapper.next(),
            Some(AnyPrimitive::Large(
                Page::<Large>::from_start_address(VirtAddr::new(0)).unwrap()
            ))
        );
        assert_eq!(
            mapper.next(),
            Some(AnyPrimitive::Large(
                Page::<Large>::from_start_address(VirtAddr::new(Large::SIZE)).unwrap()
            ))
        );
        assert_eq!(
            mapper.next(),
            Some(AnyPrimitive::Large(
                Page::<Large>::from_start_address(VirtAddr::new(2 * Large::SIZE)).unwrap()
            ))
        );
        assert_eq!(
            mapper.next(),
            Some(AnyPrimitive::Medium(
                Page::<Medium>::from_start_address(VirtAddr::new(3 * Large::SIZE)).unwrap()
            ))
        );
        assert_eq!(mapper.next(), None);
    }

    fn test_greedy_take_same() {
        let mut mapper = new(3, 3 * Large::SIZE + Medium::SIZE);

        let large1 = mapper.next().unwrap();
        let large2 = mapper.next().unwrap();
        let large3 = mapper.next().unwrap();
        let medium1 = mapper.next().unwrap();

        let mut mapper = new(3, 3 * Large::SIZE + Medium::SIZE);

        assert_eq!(mapper.try_take_same(large1), Some(large1));
        assert_eq!(mapper.try_take_same(large2), Some(large2));
        assert_eq!(mapper.try_take_same(large3), Some(large3));
        assert_eq!(mapper.try_take_same(medium1), Some(medium1));

        let mut mapper = new(0, Medium::SIZE);

        assert_eq!(
            mapper.next(),
            Some(AnyPrimitive::Medium(
                Page::from_start_address(VirtAddr::new(0)).unwrap()
            ))
        );

        let mut mapper = new(0, Medium::SIZE);

        let page = AnyPrimitive::Small(Page::<Small>::try_new_u64(0).unwrap());
        assert_eq!(mapper.try_take_same(page), Some(page));
        for i in 0..(Medium::SIZE / Small::SIZE) - 1 {
            assert_eq!(
                mapper.next(),
                Some(AnyPrimitive::Small(
                    Page::<Small>::try_new_u64((i + 1) * Small::SIZE).unwrap()
                ))
            );
        }
    }

    #[test]
    fn test_joint_fragment_mapper_optimal() {
        let mut mapper = JointFragmentMapper::new(
            VirtAddr::new(3),
            PhysAddr::new(3),
            3 * Large::SIZE + Medium::SIZE,
        );

        assert_eq!(
            mapper.next(),
            Some((
                AnyPrimitive::Large(Page::<Large>::from_start_address(VirtAddr::new(0)).unwrap()),
                AnyPrimitive::Large(Frame::<Large>::from_start_address(PhysAddr::new(0)).unwrap())
            ))
        );
        assert_eq!(
            mapper.next(),
            Some((
                AnyPrimitive::Large(
                    Page::<Large>::from_start_address(VirtAddr::new(Large::SIZE)).unwrap()
                ),
                AnyPrimitive::Large(
                    Frame::<Large>::from_start_address(PhysAddr::new(Large::SIZE)).unwrap()
                )
            ))
        );
        assert_eq!(
            mapper.next(),
            Some((
                AnyPrimitive::Large(
                    Page::<Large>::from_start_address(VirtAddr::new(2 * Large::SIZE)).unwrap()
                ),
                AnyPrimitive::Large(
                    Frame::<Large>::from_start_address(PhysAddr::new(2 * Large::SIZE)).unwrap()
                )
            ))
        );
        assert_eq!(
            mapper.next(),
            Some((
                AnyPrimitive::Medium(
                    Page::<Medium>::from_start_address(VirtAddr::new(3 * Large::SIZE)).unwrap()
                ),
                AnyPrimitive::Medium(
                    Frame::<Medium>::from_start_address(PhysAddr::new(3 * Large::SIZE)).unwrap()
                )
            ))
        );
        assert_eq!(mapper.next(), None);
    }

    #[test]
    fn test_joint_fragment_mapper_non_optimal() {
        let mut mapper =
            JointFragmentMapper::new(VirtAddr::new(0), PhysAddr::new(0x2000), Medium::SIZE);
        println!("mapper: {:?}", mapper);
        for i in 0..(Medium::SIZE / Small::SIZE) {
            assert_eq!(
                mapper.next(),
                Some((
                    AnyPrimitive::Small(
                        Page::from_start_address(VirtAddr::new(i * Small::SIZE)).unwrap()
                    ),
                    AnyPrimitive::Small(
                        Frame::from_start_address(PhysAddr::new(0x2000 + (i * Small::SIZE)))
                            .unwrap()
                    )
                ))
            );
        }
    }
}
