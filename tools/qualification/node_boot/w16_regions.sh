# Keep the authored placements available when either W16 row is selected.
w16_regions() {
  python3 - "$@" <<'PY_REGIONS'
import re
import sys

regions, groups, batch = sys.argv[1:]
regions = regions.split(";")
if not groups or "rows" in re.sub(r"\s", "", groups).split(","):
    match = re.fullmatch(r"\+?([0-9]{1,20}):\+?([0-9]{1,20})", batch)
    if batch == "all" or (
        match
        and (start := int(match[1])) < 2**64
        and (length := int(match[2])) < 2**64
        and any(start <= index < start + length for index in (34, 214))
    ):
        for region in ("128,125", "132,127", "133,126"):
            if region not in regions:
                regions.append(region)
print(";".join(regions))
PY_REGIONS
}
