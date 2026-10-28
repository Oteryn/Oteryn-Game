# Arena v1

Capability 170 exposes command 240 to an admitted session with current channel runtime authority.

## Request

The payload is empty. Challenge facts are server-owned; any payload byte is refused.

## Reply

A sequence of field 1 messages, one per Svargrond challenge, at most 1024 bytes. Each message carries varint field 1 rank (1 Greenhorn, 2 Scrapper, 3 Warlord), field 2 entry fee in gold, field 3 room count and field 4 seconds per room.
