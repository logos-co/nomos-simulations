import time
import ed25519

def verify_signatures(committee_size, depth):
    # Simulate verifying depth * committee_size signatures
    start_time = time.time()

    # Generate a single EdDSA key pair for verification
    private_key, public_key = ed25519.create_keypair()

    # Simulate depth * committee_size signature verifications
    for _ in range(depth * committee_size):
        message = b"Message to be signed"
        signature = private_key.sign(message)
        public_key.verify(signature, message)

    end_time = time.time()
    elapsed_time = end_time - start_time
    return elapsed_time

def aggregate_signatures(committee_size, root_members):
    # Simulate aggregating 3 * committee_size signatures
    start_time = time.time()

    # Generate multiple EdDSA key pairs for signature aggregation
    private_keys = [ed25519.create_keypair() for _ in range(root_members * committee_size)]

    # Simulate signature aggregation
    shared_message = b"Shared message to be signed"
    signatures = [private_key[0].sign(shared_message) for private_key in private_keys]

    # Aggregation isn't straightforward for EdDSA; we'll skip it in this example

    end_time = time.time()
    elapsed_time = end_time - start_time
    return elapsed_time

