function sum_to(n: int) -> int {
  let total = 0;
  let i = 1;

  while (i <= n) {
    total = total + i;
    i = i + 1;
  }

  total
}

function main() -> int {
  let seed = 8;
  let triangle = sum_to(seed);
  triangle
}
