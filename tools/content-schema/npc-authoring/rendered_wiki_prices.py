"""Reduce Tavily-rendered BR/Fandom shop tables to digest-bound price observations.

Raw captures stay outside the repository. Rendered template prices are observations,
not explicit wikitext prices, native admission or independent Global confirmation.
"""
import argparse
import hashlib
import json
import re
from collections import defaultdict
from pathlib import Path
from urllib.parse import unquote, urlsplit

from promotion_candidates import fold, registry_item_names, wiki_item


def digest(value):
    return hashlib.sha256(value.encode('utf-8')).hexdigest()


def links(text):
    """Markdown links with balanced URL parentheses, excluding image links."""
    pattern = re.compile(r'(?<!!)\[([^\]\n]+)\]\(')
    for match in pattern.finditer(text):
        depth, end = 1, match.end()
        while end < len(text) and depth:
            if text[end] == '(':
                depth += 1
            elif text[end] == ')':
                depth -= 1
            end += 1
        if depth:
            continue
        target = text[match.end():end - 1].split(' "', 1)[0]
        yield match.start(), end, match.group(1), target


def observed_row(text, start, end, item, price, direction):
    raw = text[start:end]
    return {'item_name': item, 'direction': direction, 'unit_price': price,
            'byte_start': len(text[:start].encode('utf-8')),
            'byte_end': len(text[:end].encode('utf-8')), 'row_sha256': digest(raw)}


def without_images(text):
    for match in list(re.finditer(r'!\[[^\]\n]*\]\(', text))[::-1]:
        depth, end = 1, match.end()
        while end < len(text) and depth:
            if text[end] == '(':
                depth += 1
            elif text[end] == ')':
                depth -= 1
            end += 1
        if not depth:
            text = text[:match.start()] + text[end:]
    return text


def br_rows(text):
    start = text.find('Itens negociáveis:')
    if start < 0:
        return []
    # Only price links inside the actual shop field; transcript mentions never count.
    end = re.search(r'\b(?:Falas|Observações|Notas|Diálogo|Transcrições|Histórico)\s*:'
                    r'|\*\*(?:Fal(?:as|a)|Diálogo|Transcri|Histórico)|\n#{1,6} '
                    r'|Menu de navegação', text[start:], re.I)
    stop = start + end.start() if end else len(text)
    article = text[start:stop]
    markers = list(re.finditer(r'\b(Vende|Compra):', article))
    rows = []
    for index, marker in enumerate(markers):
        direction = 'SellToPlayer' if marker.group(1) == 'Vende' else 'BuyFromPlayer'
        left = start + marker.end()
        right = start + markers[index + 1].start() if index + 1 < len(markers) else stop
        entries = list(links(text[left:right]))
        for item_link, currency_link in zip(entries, entries[1:]):
            a, b, name, target = item_link
            c, d, currency, _ = currency_link
            if currency.casefold() != 'gp' or '/wiki/' not in target or name.casefold() == 'gp':
                continue
            field = text[left + b:left + c]
            # Images can contain literal parentheses. Remove them through a balanced scan.
            field = without_images(field)
            field = field.strip().strip('*').strip()
            if not re.fullmatch(r'\d{1,3}(?:[ .]\d{3})+|\d+', field):
                continue
            price = int(re.sub(r'[ .]', '', field))
            row = observed_row(text, left + a, left + d, name, price, direction)
            context = text[left:left + a]
            row['scope_holds'] = (['CONDITIONAL_OR_HISTORICAL_SHOP_CONTEXT']
                                  if re.search(r'\b(?:somente|após|depois|antes|durante)\b'
                                               r'|\btendo\s+(?:o\s+)?rank\b', context, re.I) else [])
            row['context_sha256'] = digest(context)
            row['context_byte_start'] = len(text[:left].encode('utf-8'))
            row['context_byte_end'] = row['byte_start']
            rows.append(row)
    return rows


def fandom_rows(text):
    direction, rows, offset, historical = None, [], 0, False
    for line in text.splitlines(keepends=True):
        heading = line.strip().lstrip('#').strip()
        if (line.startswith('#')
                and heading.casefold().startswith(('history', 'historical', 'former', 'previous', 'old trade'))):
            historical, direction = True, None
        elif heading in ('Sells', 'Buys') and not historical:
            direction = 'SellToPlayer' if heading == 'Sells' else 'BuyFromPlayer'
        elif heading.startswith(('Transcripts', 'Notes', 'Related Pages')) or line.startswith('#'):
            direction = None
        elif direction and line.lstrip().startswith('|'):
            cells = line.strip().split('|')
            if len(cells) == 5:
                item = list(links(cells[2]))
                price = re.match(r'\s*(\d{1,3}(?:[ ,]\d{3})+|\d+)\s*!\[Image \d+: Gold\]', cells[3])
                # A second amount, pack price, or annotation must remain unresolved.
                tail = without_images(cells[3])
                images = re.findall(r'!\[([^\]]*)\]\(', cells[3])
                if (len(item) == 1 and price
                        and cells[2][:item[0][0]].strip() == ''
                        and cells[2][item[0][1]:].strip() == ''
                        and len(images) == 1
                        and re.fullmatch(r'\s*[\d ,]+\s*', tail)):
                    value = int(re.sub(r'[ ,]', '', price.group(1)))
                    rows.append(observed_row(text, offset, offset + len(line), item[0][2], value, direction))
        offset += len(line)
    return rows


def capture_facts(result, request_id):
    url, text = result['url'], result['raw_content']
    host = urlsplit(url).hostname
    if host == 'www.tibiawiki.com.br':
        source, rows = 'br', br_rows(text)
    elif host == 'tibia.fandom.com':
        source, rows = 'fandom', fandom_rows(text)
    else:
        raise ValueError('unsupported wiki source')
    name = unquote(urlsplit(url).path.removeprefix('/wiki/')).replace('_', ' ')
    # A fetched error/redirect page must not acquire the requested NPC's identity.
    headings = re.findall(r'^# ([^\n]+)', text, re.M)
    identity = len(headings) == 1 and fold(headings[0]) == fold(name)
    if not identity:
        rows = []
    revisions = sorted(set(re.findall(r'[?&]oldid=(\d+)', text)))
    return {'source': source, 'url': url, 'name': name, 'request_id': request_id,
            'capture_sha256': digest(text), 'capture_bytes': len(text.encode('utf-8')),
            'revision_hints': revisions, 'identity_heading_matches': identity,
            'price_semantics': 'RENDERED_TEMPLATE_OBSERVATION', 'rows': rows,
            'runtime_qualified': False}


def reduce_responses(paths):
    pages, failures = {}, []
    for path in paths:
        response = json.loads(Path(path).read_text(encoding='utf-8'))
        for result in response.get('results', []):
            if urlsplit(result['url']).hostname not in ('www.tibiawiki.com.br', 'tibia.fandom.com'):
                continue
            if '/wiki/Category:' in result['url']:
                continue
            page = capture_facts(result, response.get('request_id'))
            key = page['source'], fold(page['name'])
            if key in pages and pages[key]['capture_sha256'] != page['capture_sha256']:
                raise ValueError('different captures for the same wiki identity; select one explicitly')
            pages[key] = page
        failures.extend(response.get('failed_results', []))
    return {'schema': 'NPC_RENDERED_WIKI_PRICES/v1', 'runtime_qualified': False,
            'pages': sorted(pages.values(), key=lambda p: (p['source'], fold(p['name']))),
            'failed_results': sorted(failures, key=lambda p: p['url'])}


def compare_pending(pending, facts, names):
    indexes = defaultdict(list)
    for page in facts['pages']:
        for index, row in enumerate(page['rows']):
            if row.get('scope_holds'):
                continue
            indexes[fold(page['name']), row['direction'], wiki_item(row['item_name'])].append(
                {'source': page['source'], 'price': row['unit_price'],
                 'capture_sha256': page['capture_sha256'], 'url': page['url'], 'row_index': index})
    records = []
    for record in pending['records']:
        offer = record['source_offer']
        name = names.get(offer['item']['key'])
        observations = indexes[fold(record['name']), offer['direction'], wiki_item(name)] if name else []
        by_source = defaultdict(set)
        for observation in observations:
            by_source[observation['source']].add(observation['price'])
        prices = set.union(*by_source.values()) if by_source else set()
        status = ('NO_EXACT_ITEM_PRICE' if not prices else 'RENDERED_SUPPORTS_RETAINED_PRICE'
                  if prices == {offer['unit_price']} else 'RENDERED_PRICE_CONFLICT')
        # Even a unanimous rendered row does not prove per-unit, access or pack semantics.
        records.append({'npc': record['npc'], 'name': record['name'], 'service': record['service'],
                        'native_offer_tuple': record['native_offer_tuple'], 'registered_item_name': name,
                        'source_candidate_pointer': record['source_candidate_pointer'], 'status': status,
                        'historical_dissent': offer['parity_pending'], 'observations': observations,
                        'price_parity_qualified': False, 'runtime_qualified': False})
    counts = {status: sum(r['status'] == status for r in records) for status in sorted({r['status'] for r in records})}
    return {'schema': 'NPC_RENDERED_PRICE_COMPARISON/v1', 'scope': 'ALL_R5_PENDING_OFFERS',
            'global_complete': False, 'active_content_modified': False, 'counts': counts, 'records': records}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--captures', nargs='+', required=True)
    parser.add_argument('--pending', type=Path, required=True)
    parser.add_argument('--out', type=Path, required=True)
    args = parser.parse_args()
    facts = reduce_responses(args.captures)
    pending = json.loads(args.pending.read_text(encoding='utf-8'))
    comparison = compare_pending(pending, facts, registry_item_names())
    args.out.mkdir(parents=True, exist_ok=True)
    for name, value in [('rendered-price-facts.json', facts), ('price-comparison.json', comparison)]:
        (args.out / name).write_text(json.dumps(value, ensure_ascii=False, indent=2, sort_keys=True) + '\n', encoding='utf-8')
    print(json.dumps({'pages': len(facts['pages']), 'offers': len(comparison['records']), **comparison['counts']}))


if __name__ == '__main__':
    main()
