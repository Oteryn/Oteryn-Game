"""Prepare identity-bound encyclopedia Documents from captured Wiki prose.

Preserve source language and raw extracts in an independent QA receipt. Unsupported
transclusions are deferred as complete fields, never silently dropped from a field.
"""
import argparse
import gzip
import hashlib
import html
import json
import re
from collections import Counter
from pathlib import Path

from complete_optional_wiki_fields import BASELINE_SHA, BR_SHA, COVERAGE_SHA, digest, fact, load

FIELDS = ('behavior', 'notes', 'history', 'flavortext')


class UnsupportedMarkup(ValueError):
    pass


def render_template(match, helpers=None):
    helpers = helpers or {}
    parts = [p.strip() for p in match.group(1).split('|')]
    name = parts[0].casefold()
    if name == 'achievement' and len(parts) == 2:
        return parts[1]
    if name == 'dash':
        if len(parts) == 1:
            return ''
        if 'Predefinição:DASH' in helpers and len(parts) == 2:
            return parts[1]
    if name == 'mapa' and len(parts) in (2, 3) and re.fullmatch(r'\d+,\d+,\d+(?::\d+)?(?:&minimap)?', parts[1]):
        return (parts[2] + ' ' if len(parts) == 3 else '') + '(' + parts[1] + ')'
    if name == 'officialnewsarchive' and len(parts) in (2, 3) and parts[1].isdigit() and 'Predefinição:OfficialNewsArchive' in helpers:
        return (parts[2] + ' ' if len(parts) == 3 else '') + '(https://www.tibia.com/news/?subtopic=newsarchive&id=' + parts[1] + ')'
    if name == 'transcrições' and 'Predefinição:Transcrições' in helpers:
        text = next((part.removeprefix('texto=') for part in parts[1:] if part.startswith('texto=')), None)
        if text is not None:
            return text
    if name in ('#widget:youtube', '#widget:youtubeshorts'):
        video = next((part.removeprefix('id=') for part in parts[1:] if part.startswith('id=')), '')
        if re.fullmatch(r'[A-Za-z0-9_-]{11}', video):
            return 'https://www.youtube.com/' + ('shorts/' if name.endswith('youtubeshorts') else 'watch?v=') + video
    title = parts[0].removeprefix(':')
    if parts[0].startswith(':') and len(parts) == 1 and title in helpers:
        return helpers[title]['content']
    raise UnsupportedMarkup('unsupported transclusion: ' + parts[0])


def render_link(match):
    parts = match.group(1).split('|')
    target = parts[0].strip()
    if target.casefold().startswith(('arquivo:', 'file:', 'image:', 'imagem:', 'category:', 'categoria:')):
        return ''  # Non-prose illustration/category markup; raw extract is retained.
    return parts[-1] if len(parts) > 1 else target.split('#', 1)[0]


def paragraphs(raw, helpers=None):
    """Deterministic formatting removal; unknown template semantics fail closed."""
    text = re.sub(r'<!--.*?-->', '', raw, flags=re.S)
    for _ in range(5):
        if '{{' not in text:
            break
        text = re.sub(r'\{\{([^{}]+)\}\}', lambda m: render_template(m, helpers), text)
    if '{{' in text or '}}' in text:
        raise UnsupportedMarkup('nested/unbalanced transclusion')
    text = re.sub(r'<gallery[^>]*>.*?(?:</gallery>|$)', '', text, flags=re.S | re.I)
    for _ in range(5):
        old = text
        text = re.sub(r'\[\[([^\[\]]+)\]\]', render_link, text)
        if text == old:
            break
    text = re.sub(r'\[(https?://[^\s\]]+)\s+([^\]]+)\]', lambda m: m[2] + ' (' + m[1] + ')', text)
    text = re.sub(r'\[(https?://[^\s\]]+)\]', lambda m: m[1], text)
    text = re.sub(r'<(?:br\s*/?|/p|/li|/tr|/td|/th|/h[1-6])\s*>', '\n', text, flags=re.I)
    text = re.sub(r'<[^>]+>', '', text)
    text = html.unescape(text).replace("'''", '').replace("''", '')
    if '[[' in text or ']]' in text or '{|' in text or '|}' in text:
        raise UnsupportedMarkup('unsupported table/link structure')
    result = []
    for line in text.splitlines():
        line = re.sub(r'^\s*={2,6}|={2,6}\s*$', '', line).strip()
        line = re.sub(r'[ \t]+', ' ', line)
        if line:
            result.append(line)
    return result


def prepare_actor(slug, creature, dependencies, page, *, variant=False, helpers=None):
    if 'encyclopedia' in creature:
        return [], [], ['EXISTING_ENCYCLOPEDIA_PRESERVED'], []
    content, included, qa, deferred = [], [], [], []
    for field in FIELDS:
        raw = page.get('fields', {}).get(field)
        if not raw:
            continue
        receipt = {'field': field, 'source_language': 'en' if field == 'flavortext' else 'pt-BR', 'raw': raw, 'source_line': page.get('field_lines', {}).get(field)}
        try:
            lines = paragraphs(raw, helpers)
        except UnsupportedMarkup as error:
            receipt.update(status='UNRENDERED_FIELD', reason=str(error))
            deferred.append({'field': field, 'reason': str(error)})
        else:
            receipt.update(status='RENDERED_COMPLETE_FIELD', rendered_paragraphs=lines)
            if lines:
                content.extend(lines)
                included.append(field)
        qa.append(receipt)
    if not content:
        return [], qa, ['WIKI_NARRATIVE_UNAVAILABLE'], deferred
    identity = {'key': 'oteryn:document/monster-encyclopedia/' + slug,
                'revision': 'wikibr-' + str(page['revision_id'])}
    document = {'identity': identity, 'document_type': 'Report',
                'title': page['page_title'], 'author': 'TibiaWiki BR contributors',
                'language': 'en' if included == ['flavortext'] else 'pt-BR', 'content': content}
    source = fact(page, included[0])
    source['source_field'] = ','.join(included)
    source['fields'] = {f: {'raw': page['fields'][f], 'source_line': page['field_lines'].get(f)} for f in included}
    source['text_processing'] = 'Formatting-only Wiki extraction; no translation or paraphrase. Unsupported template fields separately deferred.'
    common = {'monster': slug, 'expected_present': False, 'expected_value': None,
              'source': source, 'reason': 'Actual captured Wiki narrative; source language retained. Original raw extracts and attribution remain in source QA receipt.'}
    used_helpers = [fact(helper, 'actual transcluded helper') for title, helper in (helpers or {}).items() if any(title.removeprefix('Predefinição:').casefold() in page['fields'][field].casefold() for field in included)]
    if used_helpers:
        common['supporting_sources'] = used_helpers
    rows = [{**common, 'file': 'dependencies.json', 'pointer': '/documents/-', 'value': document},
            {**common, 'file': 'monster.json', 'pointer': '/creature/encyclopedia',
             'value': {'description_document': {'family': 'Document', **identity}}}]
    flags = ['WIKI_NARRATIVE_FORMATTING_RENDERED', 'ENCYCLOPEDIA_DOCUMENT_CLIENT_DISPLAY_UNVERIFIED']
    if variant:
        flags.append('SOURCE_WIKI_NARRATIVE_SHARED_TITLE_REFERENCE')
    if deferred:
        flags.append('WIKI_NARRATIVE_SOURCE_FIELD_UNRENDERED')
    return rows, qa, flags, deferred


def run(baseline, wiki, variants_file, output, supplements=()):
    baseline, wiki, output = map(Path, (baseline, wiki, output))
    if digest(baseline / 'population-index.json') != BASELINE_SHA:
        raise ValueError('wrong baseline population index')
    if digest(wiki / 'pages.json.gz') != BR_SHA or digest(wiki / 'actor-page-coverage.json.gz') != COVERAGE_SHA:
        raise ValueError('wrong frozen Wiki source')
    helpers = {}
    for filename in supplements:
        packet = load(filename)
        for raw in json.loads(packet['text']).get('query', {}).get('pages', []):
            if 'revisions' not in raw:
                continue
            revision = raw['revisions'][0]
            content = revision['slots']['main']['content']
            title = raw['title']
            helpers[title] = {'page_title': title, 'page_id': raw['pageid'], 'revision_id': revision['revid'],
                              'content_sha256': hashlib.sha256(content.encode()).hexdigest(), 'content': content,
                              'url': 'https://tibiawiki.com.br/wiki/' + __import__('urllib.parse', fromlist=['quote']).quote(title.replace(' ', '_'), safe=''),
                              'method': packet['method'], 'retrieved_at': packet['retrieved_at']}
    index = load(baseline / 'population-index.json')
    pages = {p['page_title']: p for p in load(wiki / 'pages.json.gz')['pages']}
    coverage = {a['monster']: a for a in load(wiki / 'actor-page-coverage.json.gz')['actors']}
    variants = set(load(variants_file)['shared_title_variant_unverified_actors'])
    patches, actor_flags, qa, unavailable, counts = [], {}, [], [], Counter()
    for actor in index['monsters']:
        slug = actor['monster']
        cov = coverage[slug]
        page = pages.get(cov.get('resolved_title')) if cov['status'] == 'FRESH_PAGE_CAPTURED' else None
        if page is None or cov.get('skip_generated_variant'):
            unavailable.append({'monster': slug, 'reason': cov['status']})
            continue
        if page['content_sha256'] != cov['page_content_sha256'] or page['revision_id'] != cov['page_revision_id']:
            raise ValueError('Wiki identity mismatch: ' + slug)
        directory = baseline / 'bundles' / slug
        creature = load(directory / 'monster.json')['creature']
        dependencies = load(directory / 'dependencies.json')
        if 'documents' not in dependencies:
            raise ValueError('missing baseline documents array: ' + slug)
        rows, extracts, flags, deferred = prepare_actor(slug, creature, dependencies, page, variant=slug in variants, helpers=helpers)
        counts.update(field['field'] for field in extracts if field['status'] == 'RENDERED_COMPLETE_FIELD')
        patches.extend(rows)
        if rows:
            actor_flags[slug] = flags
        else:
            unavailable.append({'monster': slug, 'reason': flags[0]})
        qa.append({'monster': slug, 'source': fact(page, 'actual prose fields'),
                   'attribution': 'TibiaWiki BR contributors; original captured revision linked in source; exact license terms recorded separately by source collector.',
                   'variant_reference_only': slug in variants, 'fields': extracts, 'deferred': deferred})
    output.mkdir(parents=True, exist_ok=True)
    if helpers:
        with gzip.open(output / 'narrative-supplemental-pages.json.gz', 'wt', encoding='utf-8') as stream:
            json.dump({'pages': list(helpers.values())}, stream, ensure_ascii=False)
    packet = {'schema': 'OTERYN_MONSTER_FIELD_PATCH/v1', 'lane': 'wiki-encyclopedia-documents',
              'baseline_index_sha256': BASELINE_SHA, 'patches': patches, 'actor_flags': actor_flags,
              'resolved_actor_flags': {}, 'source_snapshot_sha256': BR_SHA, 'global_parity_asserted': False}
    (output / 'narrative-field-patches.json').write_text(json.dumps(packet, ensure_ascii=False, indent=2) + '\n')
    with gzip.open(output / 'narrative-source-qa.json.gz', 'wt', encoding='utf-8') as stream:
        json.dump({'actors': qa, 'unavailable': unavailable}, stream, ensure_ascii=False)
    summary = {'actors': len(index['monsters']), 'documents': len(actor_flags), 'patches': len(patches),
               'complete_rendered_field_counts': dict(counts), 'unavailable_actors': len(unavailable),
               'unrendered_fields': sum(len(a['deferred']) for a in qa),
               'qualified_shared_title_documents': sum('SOURCE_WIKI_NARRATIVE_SHARED_TITLE_REFERENCE' in f for f in actor_flags.values()),
               'packet_sha256': digest(output / 'narrative-field-patches.json'),
               'qa_sha256': digest(output / 'narrative-source-qa.json.gz'),
               'baseline_index_sha256': BASELINE_SHA, 'source_snapshot_sha256': BR_SHA,
               'runtime_qualified': False, 'production_activated': False}
    (output / 'narrative-summary.json').write_text(json.dumps(summary, ensure_ascii=False, indent=2) + '\n')
    return summary


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    for option in ('baseline', 'wiki', 'variants', 'output'):
        parser.add_argument('--' + option, required=True)
    parser.add_argument('--supplement', action='append', default=[])
    args = parser.parse_args()
    print(json.dumps(run(args.baseline, args.wiki, args.variants, args.output, args.supplement)))
