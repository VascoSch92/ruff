# errors
def print_number(number: int) -> None:
    match number:
        case 1:
            print(1)
        case 2:
            print(2)
        case 1:
            print(1)

def print_color(color: str) -> None:
    match color:
        case "red":
            print("This is red")
        case "green":
            print("this is green")
        case _:
            print("Nothing to print")
        case _:
            print("still nothing to print")

def select_color_from_number(n: int) -> None:
    match n:
        case n if n % 2 == 0:
            print("This is even")
        case n if n % 2 == 1:
            print("This is odd")
        case n if n % 2 == 0:
            print("This is even")


