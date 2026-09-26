"""Bounded helpers for already decoded source fields; not a Lua loader or runtime."""

def cast_geometry(*, length=0, spread=0, radius=None, target=False):
    """Mirror source area precedence while retaining the separate direction flag.

    In the inspected spell parser length>0 enables direction; a positive radius
    subsequently replaces the area, without clearing direction. Absent/nonpositive
    radius does not replace the length area. Input must already be source-resolved.
    """
    if not isinstance(target, bool):
        raise ValueError('target must be an explicitly resolved boolean')
    for value in (length, spread):
        if type(value) is not int or value < 0:
            raise ValueError('length and spread must be nonnegative integers')
    if radius is not None and (type(radius) is not int or radius < 0):
        raise ValueError('radius must be absent or a nonnegative integer')
    result = {'needs_target': target, 'needs_direction': length > 0}
    if length > 0:
        result['area'] = {'length_tiles': length, 'spread_tiles': spread}
    if radius is not None and radius > 0:
        result['area'] = {'radius_tiles': radius}
    return result

def health_disposition(*, health, max_health):
    """Do not silently turn zero-HP helper entities into ordinary monsters."""
    if type(health) is not int or type(max_health) is not int:
        raise ValueError('health must be explicit integer source values')
    if health == max_health == 0:
        return 'unresolved_semantics'
    if max_health <= 0 or health <= 0 or health > max_health:
        raise ValueError('invalid ordinary-monster health')
    return 'mapped'
