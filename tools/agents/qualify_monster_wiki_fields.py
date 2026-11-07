"""Offline Wiki field guard. Missing/uncertain values and mismatched titles never become facts."""
from fractions import Fraction
import re
import unicodedata

COUNT_FIELDS = frozenset({'health', 'initial_health', 'max_health', 'experience', 'armor', 'speed'})

def title_key(value):
    text = unicodedata.normalize('NFKD', str(value)).encode('ascii', 'ignore').decode().lower()
    return re.sub(r'[^a-z0-9]', '', text)

def same_title(expected_title, actual_title):
    return bool(title_key(expected_title)) and title_key(expected_title) == title_key(actual_title)

def count_value(raw, locale):
    if isinstance(raw, bool) or raw is None:
        return None
    text = str(raw).strip()
    if re.fullmatch(r'\d+', text):
        return int(text)
    # Counts are integers. Dot-grouped BR thousands are not decimal fractions.
    separator = '.' if locale == 'pt-BR' else ',' if locale == 'en' else None
    if separator and re.fullmatch(r'\d{1,3}(?:' + re.escape(separator) + r'\d{3})+', text):
        return int(text.replace(separator, ''))
    return None

def received_damage_percent(raw):
    if raw is None or isinstance(raw, bool):
        return None
    match = re.fullmatch(r'\s*(\d+(?:[.,]\d+)?)%\s*', str(raw))
    if not match:
        return None
    return Fraction(match.group(1).replace(',', '.'))

def qualify(expected_title, actual_title, field, raw, locale):
    if not same_title(expected_title, actual_title):
        return {'status': 'UNKNOWN', 'reason': 'PAGE_IDENTITY_MISMATCH', 'raw': raw}
    if field in COUNT_FIELDS:
        value = count_value(raw, locale)
    elif field == 'received_damage_percent':
        value = received_damage_percent(raw)
    else:
        return {'status': 'UNKNOWN', 'reason': 'UNSUPPORTED_FIELD', 'raw': raw}
    if value is None:
        return {'status': 'UNKNOWN', 'reason': 'MISSING_UNCERTAIN_OR_NONNUMERIC', 'raw': raw}
    if isinstance(value, Fraction):
        value = {'numerator': value.numerator, 'denominator': value.denominator}
    return {'status': 'WIKI_CONFIRMED', 'value': value, 'raw': raw}
