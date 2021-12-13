import Harness from './harness';

const harness = new Harness;
const code = `
function f(n) {
    if (n !== 0) {
        return 0;
    } else {
      return 1;
    }
}
f(0);
`;
harness.runCode(code, 1);
