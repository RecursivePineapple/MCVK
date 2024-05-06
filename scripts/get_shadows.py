#!/usr/bin/python3

import re
import os
import sys

class_filter = os.getenv("CLASS_FILTER")
operation = os.getenv("OPERATION", "shadows")
method_blacklist = os.getenv("METHOD_BLACKLIST", "").split(",")
field_blacklist = os.getenv("FIELD_BLACKLIST", "").split(",")

MODIFIERS = ["public", "private", "static", "final", "volatile", "transient", "synchronized"]

PUNCT = [
    "(",
    ")",
    ",",
    "=",
    ";",
    "[",
    "]",
    "{",
    "}",
    "<",
    ">",
    "."
]

PUNCT2 = [
    ("=", "=")
]

SYMBOL = 1
NUMBER = 2
PUNC = 3
WS = 4
NEWLINE = 5
COMMENT = 6
MLCOMMENT = 7

def is_alpha(c):
    return c >= 'A' and c <= 'Z' or c >= 'a' and c <= 'z'

def is_numeric(c):
    return c >= '0' and c <= '9'

def get_token_category(curr, next, prev):
    if curr == '*' and next == '/' and prev == MLCOMMENT:
        return MLCOMMENT, 2

    if curr in [' ', '\t']:
        return WS, 1

    if curr == '\n':
        return NEWLINE, 1

    if curr == '\r' and next == '\n':
        return NEWLINE, 2
        
    if curr == '/' and next == '/':
        return COMMENT, 2

    if is_numeric(curr):
        if prev == SYMBOL:
            return SYMBOL, 1
        else:
            return NUMBER, 1

    for c1, c2 in PUNCT2:
        if curr == c1 and next == c2:
            return PUNC, 2

    if curr in PUNCT:
        return PUNC, 1

    return SYMBOL, 1

data = sys.stdin.read()

tokens = []
current_token = get_token_category(data[0], data[1], None)
current_token_start = 0

i = 0
while i < len(data):
    cat, to_consume = get_token_category(
        data[i],
        data[i + 1] if i + 1 < len(data) else None,
        current_token
    )

    if cat != current_token or cat == PUNC or cat == NEWLINE:
        tokens.append((current_token, data[current_token_start:i]))
        current_token = cat
        current_token_start = i
    
    i += to_consume

if current_token != None:
    tokens.append((current_token, data[current_token_start:i]))

lines = []

line_start = 0

i = 0
while i < len(tokens):
    if tokens[i][0] == NEWLINE:
        i += 1
        lines.append(tokens[line_start:i])
        line_start = i
    else:
        i += 1

if line_start < i:
    lines.append(tokens[line_start:i])

last_class = None
class_indent = None
after_ctor = False
items = []

def find_generics(rest: list, start):
    i = start + 1
    count = 1
    while i < len(rest) and count > 0:
        if rest[i][1] == "<":
            count += 1
        elif rest[i][1] == ">":
            count -= 1
        i += 1

    if count == 0:
        return start, i, rest[start:i]
    else:
        return None, None, None

for line in lines:
    indent = ""

    for token in line:
        if token[0] == WS:
            indent += token[1]
        else:
            break

    line = [token for token in line if token[0] != WS]

    mod_idx = 0

    while mod_idx < len(line) and line[mod_idx][1] in MODIFIERS:
        mod_idx += 1

    mods = line[:mod_idx]
    rest = line[mod_idx:]

    type_generics = []
    name_generics = []

    if len(rest) > 2 and rest[1][1] == "<":
        start, end, g = find_generics(rest, 1)
        if end:
            del rest[start:end]
            type_generics = g
    elif len(rest) > 3 and rest[2][1] == "<":
        start, end, g = find_generics(rest, 2)
        if end:
            del rest[start:end]
            name_generics = g

    if len(rest) > 2 and rest[0][1] == "class" and rest[1][0] == SYMBOL:
        last_class = rest[1][1]
        after_ctor = False
        class_indent = len(indent)
        items.append(("class", None, (type_generics, name_generics), [mod[1] for mod in mods], rest[1][1], rest))

    if len(rest) > 1 and last_class != None and rest[0][1] == last_class and rest[1][1] == "(":
        items.append(("ctor", None, (type_generics, name_generics), [mod[1] for mod in mods], rest[0][1], rest))
        after_ctor = True

    if len(rest) >= 3 and rest[0][0] == SYMBOL and rest[1][0] == SYMBOL and rest[2][1] in ["=", ";"]:
        if not after_ctor and len(indent) == class_indent + 4:
            items.append(("field", last_class, (type_generics, name_generics), [mod[1] for mod in mods], rest[1][1], rest))

    if len(rest) >= 3 and rest[0][0] == SYMBOL and rest[1][0] == SYMBOL and rest[2][1] == "(":
        if len(indent) == class_indent + 4:
            items.append(("method", last_class, (type_generics, name_generics), [mod[1] for mod in mods], rest[1][1], rest))

if class_filter == "AUTO":
    for (itype, parent, generics, mods, name, rest) in items:
        if itype == "class":
            class_filter = name
            break

def generics_to_string(generics):
    generics = ", ".join([g[1] for g in generics if g[0] == SYMBOL])

    return (("<" + generics + ">") if len(generics) > 0 else "")

if operation == "list-fields":
    for (itype, parent, generics, mods, name, rest) in items:
        if itype == "field" and (class_filter is None or class_filter == parent):
            print(name)
if operation == "list-methods":
    for (itype, parent, generics, mods, name, rest) in items:
        if itype == "method" and (class_filter is None or class_filter == parent):
            print(name)
elif operation == "shadows":
    for (itype, parent, (type_generics, name_generics), mods, name, rest) in items:
        if class_filter is None or class_filter == parent:
            if itype == "field" and name not in field_blacklist:
                print("    @org.spongepowered.asm.mixin.Shadow")

                if "final" in mods:
                    print("    @org.spongepowered.asm.mixin.Final")

                field = [
                    *[mod for mod in mods if mod != "final"],
                    rest[0][1] + generics_to_string(type_generics),
                    name + generics_to_string(name_generics),
                ]

                print("    " + " ".join(field) + ";")
                print()
            elif itype == "method" and name not in method_blacklist:
                print("    @org.spongepowered.asm.mixin.Shadow")

                ret = rest[0][1]

                body = " return null; "

                if ret == "void":
                    body = " "
                elif ret == "boolean":
                    body = " return false; "
                elif ret in ["byte", "short", "int", "long"]:
                    body = " return 0; "
                elif ret == "float":
                    body = " return 0f; "
                elif ret == "double":
                    body = " return 0d; "

                method = [
                    *[mod for mod in mods if mod != "final"],
                    ret + generics_to_string(type_generics),
                    name + generics_to_string(name_generics),
                ]

                args_start = None
                args_end = None

                for i, token in enumerate(rest):
                    if token[1] == "(":
                        args_start = i
                    elif token[1] == ")":
                        args_end = i

                args = [token for token in rest[args_start+1:args_end] if token[1] != ","]

                types = [token[1] for token in args[::2]]
                names = [token[1] for token in args[1::2]]
                args = [f"{type} {name}" for (type, name) in zip(types, names)]

                print("    " + " ".join(method) + "(" + ", ".join(args) + ") {" + body + "}")
                print()
