import hashlib
from pathlib import Path
r=Path('/workspace/pr1437-resistances');packet=r/'docs/agents/evidence/OTV2-20261001-item-forge3332-promotion-v1.json';digest=hashlib.sha256(packet.read_bytes()).hexdigest();p=r/'apps/game-server/src/content/item_forge3332_promotion.rs';text=p.read_text();assert 'PENDING_PACKET_SHA' in text;p.write_text(text.replace('PENDING_PACKET_SHA',digest));print('packet SHA',digest)
