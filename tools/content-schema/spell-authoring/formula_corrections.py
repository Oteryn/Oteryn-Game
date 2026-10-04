"""Apply the offline r17 formula repair proposals without changing spell power or identity.

The damage revisions qualify an independently reimplemented neutral reference model. Source
alternatives stay in the manifest; this tool does not accept a model into the live Game.
"""
import formula_corrections_healing as healing
import formula_corrections_magic as magic
import formula_corrections_monk as monk
import formula_corrections_runes as runes
import formula_corrections_skill as skill

REFERENCE_REPOSITORY = 'kik-tibia/tibiatools'
REFERENCE_REVISION = 'eeed345c86a76eb00f5ff7b2f0e0b49927799bf1'
CORRECTORS = [('healing', healing.correct), ('magic', magic.correct), ('monk', monk.correct),
              ('runes', runes.correct), ('skill', skill.correct)]


def correct_formulas(name, spell_type, formulas, base_power):
    """Return replacement payloads and their distinct evidence notes; ambiguous ownership fails."""
    replacements, evidence = [], []
    for formula in formulas:
        matches = [(lane, result) for lane, correct in CORRECTORS
                   if (result := correct(name, spell_type, formula, base_power)) is not None]
        if len(matches) > 1:
            raise ValueError(f'{spell_type} {name}: formula correction has multiple owners')
        if not matches:
            replacements.append(formula)
            continue
        lane, (replacement, notes) = matches[0]
        if replacement.get('identity') != formula.get('identity'):
            raise ValueError(f'{spell_type} {name}: correction changed Formula identity')
        replacements.append(replacement)
        for note in notes:
            entry = (lane, note)
            if entry not in evidence:
                evidence.append(entry)
    return replacements, evidence
