"""Read the independently authored source cohort, excluding named main overlaps."""
import re


def declared_additions(path):
    source = re.sub(r'//[^\n]*', '', path.read_text())
    def ids(name):
        match = re.search(rf'const {name}: &\[u16\]\s*=\s*&\[([^]]+)\]', source)
        if match is None:
            raise ValueError('QUESTPC2_COHORT_DECLARATION')
        values = [int(v) for v in re.findall(r'\d+', match[1])]
        if len(set(values)) != len(values):
            raise ValueError('QUESTPC2_COHORT_DUPLICATE')
        return set(values)
    all_ids, shared = ids('IDS'), ids('SHARED_WITH_MAIN')
    if shared != {1791, 1948, 2822, 2823, 2848, 3593, 5102, 5103, 21982} or not shared <= all_ids:
        raise ValueError('QUESTPC2_MAIN_OVERLAP')
    return len(all_ids - shared)
