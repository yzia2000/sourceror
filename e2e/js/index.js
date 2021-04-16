import Harness from './harness';

const harness = new Harness;
const code = `
let x = 3;
function block() {
  x = x - 1;
  console.log(x);
}
function condition() {
  return x > 0;
}
function whileLoop(condition, block) {
  block();
  return condition() ? whileLoop(condition, block) : undefined;
}
whileLoop(condition, block);
`;
harness.runCode(code, 4);
