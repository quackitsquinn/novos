use crate::{
    align,
    paging::{
        Large, Medium, PrimitiveSize, Small,
        primitives::{Address, AnyPrimitive, MemoryFragment, PrimitiveClass},
    },
};

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

#[cfg(test)]
mod test {
    use crate::paging::{
        Address, Frame, Large, Medium, Page, PhysAddr, PrimitiveSize, VirtAddr,
        fragment::GreedyFragmentMapper,
        primitives::{AnyPrimitive, FrameClass, PageClass},
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

    #[test]
    fn test_greedy_next_fragment() {
        let mut mapper = GreedyFragmentMapper::<PageClass>::new(
            VirtAddr::new(3),
            3 * Large::SIZE + Medium::SIZE,
        );

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
}
