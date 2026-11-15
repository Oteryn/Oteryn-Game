#!/usr/bin/env python3
"""Extract verified, bounded QML style facts from explicitly supplied private inputs."""
import argparse
import ast
import hashlib
import json
import math
import operator
import pathlib
import re
import sys

VERSION = 1
PROPERTY = re.compile(r'^\s*(?:readonly\s+)?property\s+(int|real|color|string|bool|var|font)\s+(\w+)\s*:\s*(.*?)\s*;?\s*$')
ARITHMETIC = {ast.Add: operator.add, ast.Sub: operator.sub, ast.Mult: operator.mul, ast.Div: operator.truediv}


class InputError(ValueError):
    """Missing, ambiguous or unverified input; no output should be published."""


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def relative_path(value):
    path = pathlib.PurePosixPath(value)
    if not value or path.is_absolute() or pathlib.PureWindowsPath(value).is_absolute() or '..' in path.parts or '\\' in value:
        raise InputError(f'inventory/style path must be a relative POSIX path: {value}')
    return path.as_posix()


def read_inventory(path):
    data = json.loads(path.read_text(encoding='utf-8'))
    if not isinstance(data, dict):
        raise InputError('inventory must be a JSON object')
    entries = data.get('files')
    if not isinstance(entries, list):
        raise InputError('inventory must contain a files array')
    result = {}
    for entry in entries:
        if not isinstance(entry, dict) or not isinstance(entry.get('path'), str):
            raise InputError('inventory file entries require string path and sha256')
        name = relative_path(entry['path'])
        digest = entry.get('sha256')
        if not isinstance(digest, str) or not re.fullmatch('[0-9a-fA-F]{64}', digest):
            raise InputError(f'{name}: missing or invalid inventory sha256')
        if name in result:
            raise InputError(f'{name}: duplicate inventory path')
        result[name] = digest.lower()
    if not result:
        raise InputError('inventory files array is empty')
    return data.get('source_version'), result


def read_sources(root, expected):
    if not root.is_dir():
        raise InputError('source root is not a directory')
    sources, basenames = {}, {}
    resolved_root = root.resolve()
    for file in sorted(root.rglob('*')):
        if file.is_symlink():
            try:
                file.resolve().relative_to(resolved_root)
            except ValueError as exc:
                raise InputError(f'{file.relative_to(root).as_posix()}: source symlink escapes root') from exc
        if not file.is_file() or file.suffix.lower() not in ('.qml', '.js'):
            continue
        name = file.relative_to(root).as_posix()
        if name not in expected:
            raise InputError(f'{name}: source is absent from inventory')
        raw = file.read_bytes()
        normalized = raw.replace(b'\r\n', b'\n')
        digest = sha256(normalized)
        if digest != expected[name]:
            raise InputError(f'{name}: normalized SHA256 does not match inventory')
        previous = basenames.get(file.name)
        if previous and previous[1] != digest:
            raise InputError(f'{name}: conflicting duplicate basename with {previous[0]}')
        basenames[file.name] = (name, digest)
        try:
            text = normalized.decode('utf-8')
        except UnicodeDecodeError as exc:
            raise InputError(f'{name}: source is not UTF-8') from exc
        sources[name] = {'text': text, 'raw_sha256': sha256(raw),
                         'normalized_sha256': digest, 'inventory_sha256': expected[name],
                         'normalization': 'CRLF to LF only',
                         'raw_bytes': len(raw), 'normalized_bytes': len(normalized)}
    if not sources:
        raise InputError('no .qml/.js sources found')
    return sources


def strip_comments(text):
    """Remove QML/JS comments while preserving quoted strings and all line numbers."""
    out, i, quote = [], 0, None
    while i < len(text):
        char = text[i]
        if quote:
            out.append(char)
            if char == '\\' and i + 1 < len(text):
                i += 1
                out.append(text[i])
            elif char == quote:
                quote = None
        elif char in ('"', "'", '`'):
            quote = char
            out.append(char)
        elif text[i:i + 2] == '//':
            end = text.find('\n', i)
            if end == -1:
                out.extend(' ' * (len(text) - i))
                break
            out.extend(' ' * (end - i))
            i = end - 1
        elif text[i:i + 2] == '/*':
            end = text.find('*/', i + 2)
            if end == -1:
                raise InputError('unterminated block comment')
            end += 2
            out.extend('\n' if c == '\n' else ' ' for c in text[i:end])
            i = end - 1
        else:
            out.append(char)
        i += 1
    return ''.join(out)



def line_depths(text):
    """Brace depth at each line start, ignoring braces inside quoted strings."""
    result, depth, quote, escaped = [0], 0, None, False
    for char in text:
        if char == '\n':
            result.append(depth)
        if escaped:
            escaped = False
        elif quote and char == '\\':
            escaped = True
        elif quote and char == quote:
            quote = None
        elif not quote and char in ('"', "'", '`'):
            quote = char
        elif not quote and char == '{':
            depth += 1
        elif not quote and char == '}':
            depth -= 1
    return result


def constant(expression, values):
    """Resolve only finite literals, earlier local names and bounded arithmetic."""
    if len(expression) > 4096:
        raise ValueError('expression too long')
    tree = ast.parse(expression, mode='eval')
    if sum(1 for _ in ast.walk(tree)) > 256:
        raise ValueError('expression too complex')

    def bounded(value):
        if type(value) is int and value.bit_length() > 256:
            raise ValueError('integer too large')
        if type(value) is float and not math.isfinite(value):
            raise ValueError('non-finite number')
        return value

    def visit(node):
        if isinstance(node, ast.Constant) and type(node.value) in (int, float, str, bool):
            return bounded(node.value)
        if isinstance(node, ast.Name):
            if node.id in ('true', 'false'):
                return node.id == 'true'
            if node.id in values:
                return bounded(values[node.id])
        if isinstance(node, ast.UnaryOp) and isinstance(node.op, (ast.USub, ast.UAdd)):
            value = visit(node.operand)
            if type(value) in (int, float):
                return bounded(-value if isinstance(node.op, ast.USub) else value)
        if isinstance(node, ast.BinOp) and type(node.op) in ARITHMETIC:
            left, right = visit(node.left), visit(node.right)
            if type(left) in (int, float) and type(right) in (int, float):
                return bounded(ARITHMETIC[type(node.op)](left, right))
        raise ValueError('unsupported expression')

    return visit(tree.body)


def expression_fact(expression, values, source):
    result = {'expression': expression, 'source': source}
    try:
        result.update(status='resolved_constant', value=constant(expression, values))
    except (ValueError, SyntaxError, TypeError, ZeroDivisionError, OverflowError, RecursionError):
        result['status'] = 'unresolved_expression'
    return result


def extract(root, inventory, style):
    source_version, expected = read_inventory(inventory)
    sources = read_sources(root, expected)
    style = relative_path(style)
    if style not in sources:
        raise InputError(f'{style}: requested style source missing')
    tokens, fonts, values, evidence, assets = {}, {}, {}, [], []
    for name, source in sources.items():
        clean_text = strip_comments(source['text'])
        clean_lines = clean_text.splitlines()
        depths = line_depths(clean_text) if name == style else []
        for line, clean in enumerate(clean_lines, 1):
            location = {'file': name, 'line': line}
            if name == style and depths[line - 1] == 1:
                match = PROPERTY.match(clean)
                if match:
                    typ, key, expression = match.groups()
                    expression = expression.rstrip(';').strip()
                    if key in tokens:
                        raise InputError(f'{name}:{line}: duplicate style token {key}')
                    fact = expression_fact(expression, values, location)
                    fact['type'] = typ
                    if fact['status'] == 'resolved_constant' and typ == 'int':
                        value = fact['value']
                        if type(value) is float and value.is_integer():
                            fact['value'] = int(value)
                        elif type(value) is not int:
                            # Qt coercion/rounding is outside the supported arithmetic subset.
                            fact.pop('value')
                            fact.update(status='unresolved_expression', reason='integer coercion not inferred')
                    tokens[key] = fact
                    if fact['status'] == 'resolved_constant':
                        values[key] = fact['value']
                match = re.match(r'\s*(\w+)\s*\{(.*)\}\s*$', clean)
                if match and 'pixelSize:' in match[2]:
                    attrs = {}
                    for field in match[2].split(';'):
                        if ':' in field:
                            key, expression = map(str.strip, field.split(':', 1))
                            attrs[key] = expression_fact(expression, values, location)
                    if match[1] in fonts:
                        raise InputError(f'{name}:{line}: duplicate font block {match[1]}')
                    fonts[match[1]] = {'source': location, 'attributes': attrs}
            if clean.strip() and ':' in clean:
                evidence.append({'source': location, 'expression': clean.strip()})
            for match in re.finditer(r'"(/images/[^"\n]+)"', clean):
                value = match[1]
                assets.append({'source': location, 'url_or_fragment': value,
                               'complete_literal_image_url': bool(re.search(r'\.(png|jpe?g|gif)$', value))})
    return {'schema_version': VERSION, 'source_version': source_version,
            'scope': 'Verified lexical source facts only; not a QML runtime or proof of UI behavior',
            'sources': {name: {k: v for k, v in source.items() if k != 'text'} for name, source in sources.items()},
            'style_source': style, 'style_tokens': tokens, 'font_definitions': fonts,
            'rule_evidence': evidence, 'asset_references': assets,
            'limitations': [
                'No inferred or curated widget contracts are emitted; changing source revisions cannot inherit old semantics.',
                'Only single-line declarations/font blocks in the style root object are extracted; nested declarations stay rule evidence.',
                'Unresolved expressions remain verbatim; controller methods and imported QML/Qt behavior are never executed.',
                'All text rules are lexical evidence; visibility, event routing and layout negotiation need semantic review.',
                'An image URL may be a dynamic-expression fragment; complete_literal_image_url classifies only its quoted value.',
                'Source asset availability, fonts, redistribution rights and visual equivalence are not established by this tool.'
            ]}


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--sources', required=True, type=pathlib.Path, help='QML/JS root; inventory paths are relative to it')
    parser.add_argument('--inventory', required=True, type=pathlib.Path, help='JSON files[path,sha256], hashes of LF-normalized bytes')
    parser.add_argument('--style', required=True, help='Style QML path relative to --sources')
    parser.add_argument('--output', required=True, type=pathlib.Path)
    args = parser.parse_args(argv)
    try:
        result = extract(args.sources, args.inventory, args.style)
        input_paths = {args.inventory.resolve()}
        input_paths.update((args.sources / name).resolve() for name in result['sources'])
        if args.output.resolve() in input_paths:
            raise InputError('output must not overwrite a source or inventory input')
        output = json.dumps(result, ensure_ascii=False, indent=2, allow_nan=False) + '\n'
        args.output.write_text(output, encoding='utf-8', newline='\n')
    except (InputError, OSError, json.JSONDecodeError) as exc:
        print(f'input/output error: {exc}', file=sys.stderr)
        return 2
    print(json.dumps({'sources': len(result['sources']), 'style_tokens': len(result['style_tokens']),
                      'resolved_tokens': sum(t['status'] == 'resolved_constant' for t in result['style_tokens'].values()),
                      'font_definitions': len(result['font_definitions']),
                      'rule_evidence': len(result['rule_evidence'])}))
    return 0


if __name__ == '__main__':
    sys.exit(main())
