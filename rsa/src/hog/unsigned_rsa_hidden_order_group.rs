use crate::bigint::{extended_euclidean_gcd, BigInt};
use crate::hog::{UnsignedRsaGroupParams, RsaHOGError};
use num_traits::{One, Signed, Zero};

use std::{
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
};

use crate::Error;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UnsignedRsaHiddenOrderGroup<P: UnsignedRsaGroupParams> { // the group Z_M^*
    pub n: BigInt,
    _params: PhantomData<P>,
}

impl<P: UnsignedRsaGroupParams> Default for UnsignedRsaHiddenOrderGroup<P> {
    fn default() -> Self {
        Self::from_nat(BigInt::from(2))
    }
}

impl<P: UnsignedRsaGroupParams> Hash for UnsignedRsaHiddenOrderGroup<P> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.n.hash(state)
    }
}

impl<P: UnsignedRsaGroupParams> UnsignedRsaHiddenOrderGroup<P> {
    pub fn from_nat(n: BigInt) -> Self { // return group element from BigInt
        let mut a = n;
        assert!(a > BigInt::zero());
        a %= P::M.deref();
        UnsignedRsaHiddenOrderGroup {
            n: a,
            _params: PhantomData,
        }
    }

    pub fn op(&self, other: &Self) -> Self { // operation between two group elements
        let mut a = self.n.clone();
        a *= &other.n;
        a %= P::M.deref(); // mod self's modulus, dont need to use other's modulus
        UnsignedRsaHiddenOrderGroup {
            n: a,
            _params: PhantomData,
        }
    }

    pub fn identity() -> Self { // return 1
        UnsignedRsaHiddenOrderGroup {
            n: BigInt::one(),
            _params: PhantomData,
        }
    }

    pub fn generator() -> Result<Self, Error> { // return generator
        match P::G {
            Some(g) => Ok(Self::from_nat(g.deref().clone())),
            None => Err(Box::new(RsaHOGError::NotCyclic)),
        }
    }

    pub fn power(&self, e: &BigInt) -> Self { // return n^e mod M
        let r = self.n.modpow(e, P::M.deref());
        UnsignedRsaHiddenOrderGroup {
            n: r,
            _params: PhantomData,
        }
    }

    //TODO: Optimization for only calculating needed Bezout coefficient
    pub fn inverse(&self) -> Result<Self, Error> {
        let ((mut inv, _), gcd) = extended_euclidean_gcd(&self.n, P::M.deref());
        if gcd.abs() > BigInt::one() {
            return Err(Box::new(RsaHOGError::NotInvertible));
        }
        if inv < BigInt::zero() {
            inv += P::M.deref();
        }
        Ok(Self::from_nat(inv))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use num_bigint::BigInt;
    use once_cell::sync::Lazy;

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct TestJacobiRsaParams;

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct TestRsaParams;

    impl UnsignedRsaGroupParams for TestJacobiRsaParams {
        const G: Option<Lazy<BigInt>> = Some(Lazy::new(|| {
            BigInt::from_str("10247623214671719579447398827635168713468311038636791474154713239300883896181044327938883938574271844355811834681384444179071813535046505315167094216477443373311655002183105274440444776383912327883352834600696666804120507285363114192044425828676998383230158831813782314959983144676531968618965186052834450961802110403190471827027456541812365364053029125550719453620567486581333614917207747606530143194403422849197438380135177682641526814181771438519260792738867203986707180282649492913030146799652536505506805261054110045029837037371068038861573011609062145984218438792213994161185730652295694231756996080924553368144").unwrap()
        }));
        const M: Lazy<BigInt> = Lazy::new(|| {
            BigInt::from_str("23529844783153609317396348121984376131005978626757403583523509899470098115471804997231064945352065117117115100282667640961993458248263089294674968803178301998113646904983859689488884876621702945367733808344863516895699669584602977642543497014992155732168512939382421174893215435048306047308176432336331883493998983946183603485445893947938297728136653603713647712649222810949646390784388855516108907343828566675682159215105012031327592821049263589041622367295325346407265781653608617622476392342349393130840456771376679458695753622315629570896355300502946461386746836256753465827233822428899788871839984137984745107137").unwrap()
        });
    }

    impl UnsignedRsaGroupParams for TestRsaParams {
        const G: Option<Lazy<BigInt>> = None;
        const M: Lazy<BigInt> = Lazy::new(|| {
            BigInt::from_str("23529844783153609317396348121984376131005978626757403583523509899470098115471804997231064945352065117117115100282667640961993458248263089294674968803178301998113646904983859689488884876621702945367733808344863516895699669584602977642543497014992155732168512939382421174893215435048306047308176432336331883493998983946183603485445893947938297728136653603713647712649222810949646390784388855516108907343828566675682159215105012031327592821049263589041622367295325346407265781653608617622476392342349393130840456771376679458695753622315629570896355300502946461386746836256753465827233822428899788871839984137984745107137").unwrap().pow(10+1) // m=10, Z_{M^{m+1}}^*
        });
    }
    
    pub type JHog = UnsignedRsaHiddenOrderGroup<TestJacobiRsaParams>;
    pub type Hog = UnsignedRsaHiddenOrderGroup<TestRsaParams>;

    #[test]
    fn inverse_test() {
        let a = Hog::from_nat(BigInt::from(30));
        let inv_a = a.inverse().unwrap();
        assert_eq!(a.op(&inv_a).n, BigInt::from(1));

        let a = Hog::from_nat(BigInt::from(-30) + TestRsaParams::M.deref());
        let inv_a = a.inverse().unwrap();
        assert_eq!(a.op(&inv_a).n, BigInt::from(1));

        let a = JHog::from_nat(BigInt::from(30));
        let inv_a = a.inverse().unwrap();
        assert_eq!(a.op(&inv_a).n, BigInt::from(1));

        let a = JHog::from_nat(BigInt::from(-30) + TestRsaParams::M.deref());
        let inv_a = a.inverse().unwrap();
        assert_eq!(a.op(&inv_a).n, BigInt::from(1));
    }

    #[test]
    fn op_test() {
        let a = Hog::from_nat(TestRsaParams::M.deref() - BigInt::from(30));
        let b = Hog::from_nat(BigInt::from(40));
        let c = a.op(&b);
        assert_eq!(c.n, TestRsaParams::M.deref() - BigInt::from(1200));

        let a = JHog::from_nat(TestJacobiRsaParams::M.deref() - BigInt::from(30));
        let b = JHog::from_nat(BigInt::from(40));
        let c = a.op(&b);
        assert_eq!(c.n, TestJacobiRsaParams::M.deref() - BigInt::from(1200));
    }

    #[test]
    fn unsigned_test() {
        let a = Hog::from_nat(BigInt::from(30));
        let b = Hog::from_nat(BigInt::from(-30) + TestRsaParams::M.deref());
        assert_ne!(a, b);
    }
}
