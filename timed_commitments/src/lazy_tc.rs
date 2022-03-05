use ark_ec::ProjectiveCurve;
use ark_ff::{biginteger::BigInteger, PrimeField};

use crate::{
    basic_tc::{BasicTC, Comm as TCComm, Opening as TCOpening, TimeParams},
    Error, PedersenComm, PedersenParams,
};
use digest::Digest;
use num_bigint::Sign;
use rand::{CryptoRng, Rng};
use rsa::{
    bigint::{nat_to_f, BigInt},
    hog::RsaGroupParams,
    poe::{PoEParams, Proof as PoEProof},
    hash_to_prime::{HashToPrime},
};
use std::marker::PhantomData;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Comm<G: ProjectiveCurve, RsaP: RsaGroupParams> {
    ped_comm: G,
    tc_comm: TCComm<RsaP>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Opening<RsaP: RsaGroupParams, H2P: HashToPrime> {
    tc_opening: TCOpening<RsaP, H2P>,
    tc_m: Option<Vec<u8>>,
}
pub struct LazyTC<G: ProjectiveCurve, PoEP: PoEParams, RsaP: RsaGroupParams, H: Digest, H2P: HashToPrime> {
    _pedersen_g: PhantomData<G>,
    _tc: PhantomData<BasicTC<PoEP, RsaP, H, H2P>>,
}

impl<G: ProjectiveCurve, PoEP: PoEParams, RsaP: RsaGroupParams, H: Digest, H2P: HashToPrime>
    LazyTC<G, PoEP, RsaP, H, H2P>
{
    pub fn gen_pedersen_params<R: CryptoRng + Rng>(rng: &mut R) -> PedersenParams<G> {
        PedersenComm::<G>::gen_pedersen_params(rng)
    }

    pub fn gen_time_params(t: u32) -> Result<(TimeParams<RsaP>, PoEProof<RsaP, H2P>), Error> {
        BasicTC::<PoEP, RsaP, H, H2P>::gen_time_params(t)
    }

    pub fn ver_time_params(
        pp: &TimeParams<RsaP>,
        proof: &PoEProof<RsaP, H2P>,
    ) -> Result<bool, Error> {
        BasicTC::<PoEP, RsaP, H, H2P>::ver_time_params(pp, proof)
    }

    pub fn commit<R: CryptoRng + Rng>(
        rng: &mut R,
        time_pp: &TimeParams<RsaP>,
        ped_pp: &PedersenParams<G>,
        m: &[u8],
        ad: &[u8],
    ) -> Result<(Comm<G, RsaP>, Opening<RsaP, H2P>), Error> {
        let (ped_comm, ped_opening) = PedersenComm::<G>::commit(rng, ped_pp, m)?;
        let mut tc_m = m.to_vec();
        tc_m.append(&mut ped_opening.into_repr().to_bytes_le());
        let (tc_comm, tc_opening) = BasicTC::<PoEP, RsaP, H, H2P>::commit(rng, time_pp, &tc_m, ad)?;
        Ok((
            Comm { ped_comm, tc_comm },
            Opening {
                tc_opening,
                tc_m: Some(tc_m),
            },
        ))
    }

    pub fn force_open(
        time_pp: &TimeParams<RsaP>,
        ped_pp: &PedersenParams<G>,
        comm: &Comm<G, RsaP>,
        ad: &[u8],
    ) -> Result<(Option<Vec<u8>>, Opening<RsaP, H2P>), Error> {
        let (tc_m, tc_opening) = BasicTC::<PoEP, RsaP, H, H2P>::force_open(time_pp, &comm.tc_comm, ad)?;
        match &tc_m {
            Some(tc_m_inner) => {
                let mut m = tc_m_inner.to_vec();
                let f_bytes = <G::ScalarField as PrimeField>::BigInt::NUM_LIMBS * 8;
                let ped_opening = nat_to_f(&BigInt::from_bytes_le(
                    Sign::Plus,
                    &m.split_off(m.len() - f_bytes),
                ))?;
                let ped_valid =
                    PedersenComm::<G>::ver_open(ped_pp, &comm.ped_comm, &m, &ped_opening)?;
                if ped_valid {
                    Ok((Some(m), Opening { tc_opening, tc_m }))
                } else {
                    Ok((None, Opening { tc_opening, tc_m }))
                }
            }
            None => Ok((None, Opening { tc_opening, tc_m })),
        }
    }

    pub fn ver_open(
        time_pp: &TimeParams<RsaP>,
        ped_pp: &PedersenParams<G>,
        comm: &Comm<G, RsaP>,
        ad: &[u8],
        m: &Option<Vec<u8>>,
        opening: &Opening<RsaP, H2P>,
    ) -> Result<bool, Error> {
        let tc_valid = BasicTC::<PoEP, RsaP, H, H2P>::ver_open(
            time_pp,
            &comm.tc_comm,
            ad,
            &opening.tc_m,
            &opening.tc_opening,
        )?;
        match &opening.tc_m {
            Some(tc_m) => {
                let mut m_computed = tc_m.to_vec();
                let f_bytes = <G::ScalarField as PrimeField>::BigInt::NUM_LIMBS * 8;
                let ped_opening = nat_to_f(&BigInt::from_bytes_le(
                    Sign::Plus,
                    &m_computed.split_off(m_computed.len() - f_bytes),
                ))?;
                let ped_valid =
                    PedersenComm::<G>::ver_open(ped_pp, &comm.ped_comm, &m_computed, &ped_opening)?;
                match m {
                    Some(m) => Ok(tc_valid && ped_valid && m_computed == *m),
                    None => Ok(tc_valid && !ped_valid),
                }
            }
            None => Ok(tc_valid && m.is_none()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::G1Projective as G;
    use once_cell::sync::Lazy;
    use rand::{rngs::StdRng, SeedableRng};
    use sha3::Sha3_256;
    use std::str::FromStr;
    use rsa::hash_to_prime::pocklington::{PocklingtonHash, PocklingtonCertParams};

    use rsa::hog::RsaHiddenOrderGroup;

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct TestRsaParams;

    impl RsaGroupParams for TestRsaParams {
        const G: Lazy<BigInt> = Lazy::new(|| BigInt::from(2));
        const M: Lazy<BigInt> = Lazy::new(|| {
            BigInt::from_str("2519590847565789349402718324004839857142928212620403202777713783604366202070\
                          7595556264018525880784406918290641249515082189298559149176184502808489120072\
                          8449926873928072877767359714183472702618963750149718246911650776133798590957\
                          0009733045974880842840179742910064245869181719511874612151517265463228221686\
                          9987549182422433637259085141865462043576798423387184774447920739934236584823\
                          8242811981638150106748104516603773060562016196762561338441436038339044149526\
                          3443219011465754445417842402092461651572335077870774981712577246796292638635\
                          6373289912154831438167899885040445364023527381951378636564391212010397122822\
                          120720357").unwrap()
        });
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct TestPoEParams;

    impl PoEParams for TestPoEParams {
        const HASH_TO_PRIME_ENTROPY: usize = 128;
    }

    #[derive(Clone, PartialEq, Eq, Debug)]
    pub struct TestPocklingtonParams;
    impl PocklingtonCertParams for TestPocklingtonParams {
        const NONCE_SIZE: usize = 16;
        const MAX_STEPS: usize = 5;
    }

    pub type TC = LazyTC<G, TestPoEParams, TestRsaParams, Sha3_256, PocklingtonHash<TestPocklingtonParams, Sha3_256>>;

    #[test]
    fn lazy_tc_test() {
        let mut rng = StdRng::seed_from_u64(0u64);
        let mut m = [1u8; 8];
        rng.fill(&mut m);
        let mut ad = [0u8; 32];
        rng.fill(&mut ad);

        let (time_pp, _) = TC::gen_time_params(40).unwrap();

        let ped_pp = TC::gen_pedersen_params(&mut rng);

        let (comm, self_opening) = TC::commit(&mut rng, &time_pp, &ped_pp, &m, &ad).unwrap();
        assert!(TC::ver_open(
            &time_pp,
            &ped_pp,
            &comm,
            &ad,
            &Some(m.to_vec()),
            &self_opening
        )
        .unwrap());

        let (force_m, force_opening) = TC::force_open(&time_pp, &ped_pp, &comm, &ad).unwrap();
        assert!(TC::ver_open(&time_pp, &ped_pp, &comm, &ad, &force_m, &force_opening).unwrap());
        assert_eq!(force_m, Some(m.to_vec()));

        // Bad message
        let mut m_bad = m.to_vec();
        m_bad[0] = m_bad[0] + 1u8;
        assert!(!TC::ver_open(
            &time_pp,
            &ped_pp,
            &comm,
            &ad,
            &Some(m_bad.to_vec()),
            &self_opening
        )
        .unwrap());
        assert!(!TC::ver_open(
            &time_pp,
            &ped_pp,
            &comm,
            &ad,
            &Some(m_bad.to_vec()),
            &force_opening
        )
        .unwrap());
        assert!(!TC::ver_open(&time_pp, &ped_pp, &comm, &ad, &None, &self_opening).unwrap());
        assert!(!TC::ver_open(&time_pp, &ped_pp, &comm, &ad, &None, &force_opening).unwrap());

        // Bad associated data
        let mut ad_bad = ad.to_vec();
        ad_bad[0] = ad_bad[0] + 1u8;
        assert!(!TC::ver_open(
            &time_pp,
            &ped_pp,
            &comm,
            &ad_bad,
            &Some(m.to_vec()),
            &self_opening
        )
        .unwrap());
        assert!(
            !TC::ver_open(&time_pp, &ped_pp, &comm, &ad_bad, &force_m, &force_opening).unwrap()
        );

        // Bad commitment
        let mut tc_input_group_element_bad = comm.clone();
        tc_input_group_element_bad.tc_comm.x = RsaHiddenOrderGroup::from_nat(BigInt::from(2));
        let (force_m_bad, force_opening_bad) =
            TC::force_open(&time_pp, &ped_pp, &tc_input_group_element_bad, &ad).unwrap();
        assert!(force_m_bad.is_none());
        assert!(TC::ver_open(
            &time_pp,
            &ped_pp,
            &tc_input_group_element_bad,
            &ad,
            &force_m_bad,
            &force_opening_bad
        )
        .unwrap());

        let mut tc_ae_ct_bad = comm.clone();
        tc_ae_ct_bad.tc_comm.ct[0] += 1u8;
        let (force_m_bad, force_opening_bad) =
            TC::force_open(&time_pp, &ped_pp, &tc_ae_ct_bad, &ad).unwrap();
        assert!(force_m_bad.is_none());
        assert!(TC::ver_open(
            &time_pp,
            &ped_pp,
            &tc_ae_ct_bad,
            &ad,
            &force_m_bad,
            &force_opening_bad
        )
        .unwrap());

        let mut ped_comm_bad = comm.clone();
        ped_comm_bad.ped_comm = ped_pp.g.clone();
        let (force_m_bad, force_opening_bad) =
            TC::force_open(&time_pp, &ped_pp, &ped_comm_bad, &ad).unwrap();
        assert!(force_m_bad.is_none());
        assert!(TC::ver_open(
            &time_pp,
            &ped_pp,
            &ped_comm_bad,
            &ad,
            &force_m_bad,
            &force_opening_bad
        )
        .unwrap());
    }
}
