use cosmwasm_std::{
    entry_point, to_binary, Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult,
};

use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, MigrateMsg, QueryMsg};
use crate::state::{Config, CONFIG};
use fff::Field;
use groupy::CurveAffine;
use paired::bls12_381::{Bls12, Fq12, G1Affine, G2Affine};
use paired::{Engine, PairingCurveAffine};

use crate::error::ContractError;
use drand_verify::{derive_randomness, g1_from_variable, g2_from_variable, VerificationError};

// Note, you can use StdResult in some functions where you do not
// make use of the custom errors
#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    let config = Config {
        drand_public_key: vec![
            134, 143, 0, 94, 184, 230, 228, 202, 10, 71, 200, 167, 124, 234, 165, 48, 154, 71, 151,
            138, 124, 113, 188, 92, 206, 150, 54, 107, 93, 122, 86, 153, 55, 197, 41, 238, 218,
            102, 199, 41, 55, 132, 169, 64, 40, 1, 175, 49,
        ]
        .into(),
    };
    CONFIG.save(deps.storage, &config)?;

    Ok(Response::default())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    match msg {
        ExecuteMsg::Verify {
            signature,
            msg_g2,
            worker,
            round,
        } => verify(deps, env, info, signature, msg_g2, worker, round),
    }
}

fn fast_pairing_equality(p: &G1Affine, q: &G2Affine, r: &G1Affine, s: &G2Affine) -> bool {
    fn e_prime(p: &G1Affine, q: &G2Affine) -> Fq12 {
        Bls12::miller_loop([(&(p.prepare()), &(q.prepare()))].iter())
    }

    let minus_p = {
        let mut out = *p;
        out.negate();
        out
    };
    let mut tmp = e_prime(&minus_p, &q);
    tmp.mul_assign(&e_prime(r, &s));
    match Bls12::final_exponentiation(&tmp) {
        Some(value) => value == Fq12::one(),
        None => false,
    }
}

fn verify_step2(
    pk: &G1Affine,
    signature: &[u8],
    msg_on_g2: &G2Affine,
) -> Result<bool, VerificationError> {
    let g1 = G1Affine::one();
    let sigma = match g2_from_variable(signature) {
        Ok(sigma) => sigma,
        Err(err) => {
            return Err(VerificationError::InvalidPoint {
                field: "signature".into(),
                msg: err.to_string(),
            })
        }
    };
    Ok(fast_pairing_equality(&g1, &sigma, pk, msg_on_g2))
}

pub fn verify(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    signature: Binary,
    msg_g2: Binary,
    worker: String,
    round: u64,
) -> Result<Response, ContractError> {
    // Load config
    let config = CONFIG.load(deps.storage)?;
    // To affine
    let pk_to_g1affine = g1_from_variable(config.drand_public_key.as_slice()).unwrap();
    let msg_to_g2affine = g2_from_variable(&msg_g2.as_slice()).unwrap();
    // Verify
    let is_valid = verify_step2(&pk_to_g1affine, &signature.as_slice(), &msg_to_g2affine).unwrap();
    let randomness = derive_randomness(signature.as_slice());

    if !is_valid {
        return Err(ContractError::Unauthorized {});
    }

    Ok(Response::new()
        .add_attribute("round", round.to_string())
        .add_attribute("randomness", Binary::from(randomness).to_string())
        .add_attribute("worker", worker))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let response = match msg {
        QueryMsg::Config {} => to_binary(&query_config(deps)?)?,
    };
    Ok(response)
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(config)
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(_deps: DepsMut, _env: Env, _msg: MigrateMsg) -> StdResult<Response> {
    Ok(Response::default())
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use hex;

    #[test]
    fn verify_test() {
        let mut deps = mock_dependencies(&[]);
        let init_msg = InstantiateMsg {};
        let env = mock_env();
        let info = mock_info("sender", &[]);
        instantiate(deps.as_mut(), env.clone(), info.clone(), init_msg).unwrap();

        let signature: Binary = hex::decode("a75c1b05446c28e9babb078b5e4887761a416b52a2f484bcb388be085236edacc72c69347cb533da81e01fe26f1be34708855b48171280c6660e2eb736abe214740ce696042879f01ba5613808a041b54a80a43dadb5a6be8ed580be7e3f546e").unwrap().into();
        let g2_binary = hex::decode("8332743e3c325954435e289d757183e9d3d0b64055cf7f8610b0823d6fd2c0ec2a9ce274fd2eec85875225f89dcdda710fb11cce31d0fa2b4620bbb2a2147502f921ceb95d29b402b55b69b609e51bb759f94c32b7da12cb91f347b12740cb52").unwrap();
        println!("{}, {:?}", signature, g2_binary);
        let msg = ExecuteMsg::Verify {
            signature: signature,
            msg_g2: Binary::from(g2_binary),
            worker: "address".to_string(),
            round: 12323,
        };
        let res = execute(deps.as_mut(), env, info, msg).unwrap();
        println!("{:?}", res);
        assert_eq!(3, res.attributes.len());
    }
}
