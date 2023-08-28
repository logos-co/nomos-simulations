import time
import ecdsa

def verify_signatures(committee_size, depth):
    # Simulate verifying depth * committee_size signatures
    start_time = time.time()

    # ECDSA key generation
    private_key = ecdsa.SigningKey.generate()
    public_key = private_key.get_verifying_key()

    # Simulate depth * committee_size signature verifications
    for _ in range(depth * committee_size):
        message = b"Message to be signed"
        signature = private_key.sign(message)
        public_key.verify(signature, message)

    end_time = time.time()
    elapsed_time = end_time - start_time
    return elapsed_time



def aggregate_signatures(committee_size):
    # Simulate aggregating 3 * committee_size signatures
    start_time = time.time()

    # ECDSA key generation
    private_keys = [ecdsa.SigningKey.generate() for _ in range(3 * committee_size)]
   # public_keys = [private_key.get_verifying_key() for private_key in private_keys]

    # Simulate signature aggregation
    shared_message = b"Shared message to be signed"
    signatures = [private_key.sign(shared_message) for private_key in private_keys]
    aggregated_signature = sum(signatures, ecdsa.util.numbertheory.ordercurve.order)

    # Verify aggregated signature against the shared message
    public_key = ecdsa.VerifyingKey.from_public_point(aggregated_signature, curve=ecdsa.SECP256k1)
    assert public_key.verify(aggregated_signature, shared_message)

    end_time = time.time()
    elapsed_time = end_time - start_time
    return elapsed_time
