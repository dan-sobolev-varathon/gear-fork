use super::*;
use core::marker::PhantomData;

pub struct Actor<T: Config>(PhantomData<T>);

impl<T: Config> BuiltinActor for Actor<T> {
    const ID: u64 = 118; // 0x1ef25efb2be22235d221e0570bf57efd2b5483a39088cff6e9144b1125696632

    type Error = BuiltinActorError;

    fn handle(dispatch: &StoredDispatch, _gas_limit: u64) -> (Result<Payload, Self::Error>, u64) {
        let message = dispatch.message();
        let payload = message.payload_bytes();
        
        let gas_spent = 1_000_000_000;

        let res = gear_runtime_interface::risc_0_verifier::risc0_verifier(payload);
        let result = res.map(|a| a.try_into().unwrap()).map_err(|e| BuiltinActorError::Custom(e));

        (
            result,
            gas_spent,
        )
    }
}