use halo2_proofs::circuit::Value;
use pasta_curves::{group::ff::PrimeField, pallas};
use wasm_bindgen::prelude::*;
use zkbridge_proofs::{
    circuits::{
        common::{
            poseidon_commit_domain, poseidon_hash_offchain, poseidon_nullifier_domain,
            SHIELD_SIGNATURE_CONTEXT, UNSHIELD_SIGNATURE_CONTEXT,
        },
        shielding::ShieldingCircuit,
        unshielding::UnshieldingCircuit,
    },
    generate_artifacts,
    signing::{scalar_from_base, sign_message},
    BridgeKeyArtifacts, ProofGenerator,
};

fn field_from_bytes(bytes: &[u8]) -> Option<pallas::Base> {
    if bytes.len() != 32 {
        return None;
    }
    let mut repr = <pallas::Base as PrimeField>::Repr::default();
    repr.as_mut().copy_from_slice(bytes);
    pallas::Base::from_repr(repr).into_option()
}

fn field_to_bytes(field: &pallas::Base) -> Vec<u8> {
    field.to_repr().as_ref().to_vec()
}

#[wasm_bindgen]
pub struct ShieldingResult {
    proof: Vec<u8>,
    commitment: Vec<u8>,
    nullifier: Vec<u8>,
}

#[wasm_bindgen]
impl ShieldingResult {
    #[wasm_bindgen(getter)]
    pub fn proof(&self) -> Vec<u8> {
        self.proof.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn commitment(&self) -> Vec<u8> {
        self.commitment.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn nullifier(&self) -> Vec<u8> {
        self.nullifier.clone()
    }
}

#[wasm_bindgen]
pub struct UnshieldingResult {
    proof: Vec<u8>,
    nullifier: Vec<u8>,
}

#[wasm_bindgen]
impl UnshieldingResult {
    #[wasm_bindgen(getter)]
    pub fn proof(&self) -> Vec<u8> {
        self.proof.clone()
    }
    #[wasm_bindgen(getter)]
    pub fn nullifier(&self) -> Vec<u8> {
        self.nullifier.clone()
    }
}

#[wasm_bindgen]
pub struct ZkBridgeContext {
    artifacts: BridgeKeyArtifacts,
}

#[wasm_bindgen]
impl ZkBridgeContext {
    #[wasm_bindgen(constructor)]
    pub fn new(k: u32) -> Result<ZkBridgeContext, JsValue> {
        let artifacts = generate_artifacts(k)
            .map_err(|e| JsValue::from_str(&format!("Failed to generate artifacts: {:?}", e)))?;
        Ok(ZkBridgeContext { artifacts })
    }

    pub fn prove_shielding(
        &self,
        public_utxo_bytes: &[u8],
        amount: u64,
        private_key_bytes: &[u8],
        blinding_bytes: &[u8],
    ) -> Result<ShieldingResult, JsValue> {
        let utxo_field = field_from_bytes(public_utxo_bytes).ok_or("Invalid public UTXO")?;
        let private_key_field = field_from_bytes(private_key_bytes).ok_or("Invalid private key")?;
        let blinding_field = field_from_bytes(blinding_bytes).ok_or("Invalid blinding factor")?;
        let amount_field = pallas::Base::from(amount);

        let domain_commit = poseidon_commit_domain();
        let domain_nullifier = poseidon_nullifier_domain();

        let commitment_field =
            poseidon_hash_offchain::<4>([domain_commit, amount_field, blinding_field, utxo_field]);
        let nullifier_field = poseidon_hash_offchain::<4>([
            domain_nullifier,
            private_key_field,
            blinding_field,
            utxo_field,
        ]);

        let private_scalar = scalar_from_base(&private_key_field);
        let commitment_scalar = scalar_from_base(&commitment_field);
        let shield_signature = sign_message(
            &private_scalar,
            &commitment_scalar,
            SHIELD_SIGNATURE_CONTEXT,
        );

        let circuit = ShieldingCircuit {
            private_key: Value::known(private_key_field),
            signature_r: Value::known(shield_signature.r),
            signature_s: Value::known(shield_signature.s),
            blinding_factor: Value::known(blinding_field),
            public_utxo_id: utxo_field,
            amount: amount_field,
            output_commitment: commitment_field,
            nullifier: nullifier_field,
        };

        let proof_generator = ProofGenerator::from_params(self.artifacts.params.clone());
        let public_inputs = vec![utxo_field, amount_field, commitment_field, nullifier_field];

        let proof = proof_generator
            .generate_proof(
                circuit,
                &[public_inputs.as_slice()],
                &self.artifacts.shielding_pk,
            )
            .map_err(|e| JsValue::from_str(&format!("Proof generation failed: {:?}", e)))?;

        Ok(ShieldingResult {
            proof,
            commitment: field_to_bytes(&commitment_field),
            nullifier: field_to_bytes(&nullifier_field),
        })
    }

    pub fn prove_unshielding(
        &self,
        private_utxo_commitment_bytes: &[u8],
        amount: u64,
        private_key_bytes: &[u8],
        public_recipient_bytes: &[u8],
        origin_public_utxo_id_bytes: &[u8],
    ) -> Result<UnshieldingResult, JsValue> {
        let existing_commitment_field =
            field_from_bytes(private_utxo_commitment_bytes).ok_or("Invalid commitment")?;
        let private_field = field_from_bytes(private_key_bytes).ok_or("Invalid private key")?;
        let recipient_field =
            field_from_bytes(public_recipient_bytes).ok_or("Invalid recipient")?;
        let origin_utxo_field =
            field_from_bytes(origin_public_utxo_id_bytes).ok_or("Invalid origin UTXO")?;
        let amount_field = pallas::Base::from(amount);

        // Recipient logic: recipient = amount + blinding
        // Therefore: blinding = recipient - amount
        let blinding_field = recipient_field - amount_field;

        let domain_nullifier = poseidon_nullifier_domain();
        let nullifier_field = poseidon_hash_offchain::<4>([
            domain_nullifier,
            private_field,
            blinding_field,
            origin_utxo_field,
        ]);

        let private_scalar = scalar_from_base(&private_field);
        let nullifier_scalar = scalar_from_base(&nullifier_field);
        let unshield_signature = sign_message(
            &private_scalar,
            &nullifier_scalar,
            UNSHIELD_SIGNATURE_CONTEXT,
        );

        let circuit = UnshieldingCircuit {
            private_key: Value::known(private_field),
            signature_r: Value::known(unshield_signature.r),
            signature_s: Value::known(unshield_signature.s),
            amount: Value::known(amount_field),
            blinding_factor: Value::known(blinding_field),
            origin_utxo_id: Value::known(origin_utxo_field),
            existing_commitment: Value::known(existing_commitment_field),
            nullifier: nullifier_field,
            public_recipient: recipient_field,
            revealed_amount: amount_field,
        };

        let proof_generator = ProofGenerator::from_params(self.artifacts.params.clone());
        let public_inputs = vec![nullifier_field, recipient_field, amount_field];

        let proof = proof_generator
            .generate_proof(
                circuit,
                &[public_inputs.as_slice()],
                &self.artifacts.unshielding_pk,
            )
            .map_err(|e| JsValue::from_str(&format!("Unshielding proof failed: {:?}", e)))?;

        Ok(UnshieldingResult {
            proof,
            nullifier: field_to_bytes(&nullifier_field),
        })
    }
}
