"""Resolve a finite SOURCE reward selector alias, never the runtime selected value."""
import re

from lua_writers import mask_code, static_name_is_immutable

SELECTOR = re.compile(r'\w+\[\w+\.(?:uid|itemid)\]')


def selector_expression(script, expression, number):
    expression = expression.strip()
    if SELECTOR.fullmatch(expression):
        return expression
    if not re.fullmatch(r'\w+', expression) or not static_name_is_immutable(script.lines, expression):
        return None
    scope = script.line_scopes.get(number)
    if scope is None or any(part[0] == 'opaque' for part in scope):
        return None
    declarations = []
    for line, declared_scope in script.line_scopes.items():
        match = re.fullmatch(r'local\s+' + re.escape(expression) + r'\s*=\s*(.+)', script.raw(line))
        if match:
            declarations.append((line, declared_scope, match[1].strip()))
    if len(declarations) != 1:
        return None
    line, declared_scope, value = declarations[0]
    if (line >= number or scope[:len(declared_scope)] != declared_scope
            or any(part[0] == 'opaque' for part in declared_scope)
            or not SELECTOR.fullmatch(value)):
        return None
    selector = re.fullmatch(r'\w+\[(\w+)\.(?:uid|itemid)\]', value)[1]
    # The alias captures an earlier value; live source predicates are equivalent
    # only while no earlier callback/closure method could mutate that value.
    reads = {'getId', 'getActionId', 'getUniqueId', 'getAttribute', 'isItem', 'getType'}
    for current in script.line_scopes:
        if current <= number:
            calls = re.findall(r'(?<![\w.:])' + re.escape(selector) + r':(\w+)\s*\(',
                               mask_code(script.raw(current)))
            if any(method not in reads for method in calls):
                return None
    return value
