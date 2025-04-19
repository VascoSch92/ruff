num = 1337

def return_num() -> int:
    return num

print(oct(num)[2:])  # FURB116
print(hex(num)[2:])  # FURB116
print(bin(num)[2:])  # FURB116

print(oct(1337)[2:])  # FURB116
print(hex(1337)[2:])  # FURB116
print(bin(1337)[2:])  # FURB116

print(bin(return_num())[2:])  # FURB116 (no autofix)
print(bin(int(f"{num}"))[2:])  # FURB116 (no autofix)

# invalid
print(oct(0o1337)[1:])
print(hex(0x1337)[3:])

# From issue https://github.com/astral-sh/ruff/issues/16472

# the fix should be marked unsafe when we cannot prove that the arg
# is a pos `int` or `bool`

# safe
print(bin(1)[2:])
print(bin(True)[2:])

# unsafe
print(bin(-1)[2:])

# unsafe
import datetime
d = datetime.datetime.now(tz=datetime.UTC)
print(bin(d)[2:])

# float and complex number literal must be ignored by the rule
print(bin(1.0)[2:])
print(bin(complex(3, 4))[2:])
print(bin(3 + 4j)[2:])

