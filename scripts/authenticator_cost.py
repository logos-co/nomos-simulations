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
