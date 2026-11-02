"""Gzip containers whose bytes do not depend on the Python version."""
import gzip

# RFC 1952 header byte 9 (OS). With mtime=0, gzip.compress lets zlib write the header on
# Python 3.11 and 3.12 (OS 3, Unix), but Python 3.13 writes the header itself with OS 255.
# Pinning the byte keeps committed captures byte-identical under every interpreter, so
# `--check` replays compare equal wherever the samples were regenerated.
OS_BYTE = 3


def stable_gzip(data, level=9):
    """gzip.compress(data, level, mtime=0) with the OS header byte fixed to OS_BYTE."""
    packed = bytearray(gzip.compress(data, compresslevel=level, mtime=0))
    packed[9] = OS_BYTE
    return bytes(packed)
