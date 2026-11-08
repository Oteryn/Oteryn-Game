"""Index exact legacy aliases without inventing aliases for authored quests."""


def bindings(claims):
    result = {}
    for claim in claims:
        proof = claim['provenance']
        if 'pilot_key' not in proof or 'pilot_revision' not in proof:
            if (claim.get('quest', {}).get('key', '').startswith('oteryn:quest.')
                    and proof.get('source_packet') and proof.get('source_sha256')
                    and 'pilot_key' not in proof and 'pilot_revision' not in proof):
                continue
            raise ValueError('RewardClaim lacks exact source identity')
        identity = (proof['pilot_key'], proof['pilot_revision'])
        if identity in result:
            raise ValueError('duplicate RewardClaim source identity')
        result[identity] = claim
    return result
