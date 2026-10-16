export type FormatJsonOptions = {
  limit?: number;
  indent?: number | string;
};

export function formatJson(s: string, opts: FormatJsonOptions = {}): string {
  const limit = opts.limit ?? 100;
  const indent = typeof opts.indent === "number" ? " ".repeat(opts.indent) : (opts.indent ?? "\t");

  const n = s.length;
  const close: Record<number, number> = {};
  const lens: Record<number, number> = {};
  const stack: number[] = [];

  let quote = false;
  let esc = false;

  for (let i = 0; i < n; i++) {
    const c = s[i];
    const top = stack[stack.length - 1];

    if (quote) {
      lens[top]++;
      if (esc) esc = false;
      else if (c === "\\") esc = true;
      else if (c === '"') quote = false;
    } else if (c === '"') {
      if (top !== undefined) lens[top]++;
      quote = true;
    } else if (c === "{" || c === "[") {
      stack.push(i);
      lens[i] = 1;
    } else if (c === "}" || c === "]") {
      lens[top] = lens[top] + 1;
      close[top] = i;
      stack.pop();

      const parent = stack[stack.length - 1];
      if (parent !== undefined) lens[parent] += lens[top];
    } else if (top !== undefined && !/\s/.test(c)) {
      lens[top]++;
      if (c === "," || c === ":") lens[top]++;
    }
  }

  function compact(a: number, b: number): string {
    const out: string[] = [];
    let q = false;
    let e = false;

    for (let i = a; i <= b; i++) {
      const c = s[i];
      if (q) {
        out.push(c);
        if (e) e = false;
        else if (c === "\\") e = true;
        else if (c === '"') q = false;
      } else if (c === '"') {
        q = true;
        out.push(c);
      } else if (!/\s/.test(c)) {
        out.push(c);
        if (c === "," || c === ":") out.push(" ");
      }
    }
    return out.join("");
  }

  function stringEnd(i: number, b: number): number {
    let e = false;
    i++;
    while (i <= b) {
      const c = s[i];

      if (e) e = false;
      else if (c === "\\") e = true;
      else if (c === '"') return i;

      i++;
    }
    return b;
  }

  function fmt(a: number, b: number, depth: number): string {
    const c = s[a];

    if ((c === "{" || c === "[") && lens[a] <= limit) {
      const ending = close[a];
      if (c === "{" && ending > a + 1) return "{ " + compact(a + 1, ending - 1) + " }";
      else return compact(a, ending);
    }
    if (c !== "{" && c !== "[") return compact(a, b);

    const ending = close[a];
    const result: string[] = [c];
    const d = depth + 1;
    let i = a + 1;

    while (i < ending) {
      while (i < ending && /\s/.test(s[i])) i++;
      if (i >= ending) break;

      result.push("\n" + indent.repeat(d));

      if (c === "{") {
        const j = stringEnd(i, ending);
        result.push(s.slice(i, j + 1) + ": ");
        i = j + 1;
        while (i < ending && /\s/.test(s[i])) i++;
        i++;
        while (i < ending && /\s/.test(s[i])) i++;
      }

      const v = s[i];
      let j: number;

      if (v === "{" || v === "[") j = close[i];
      else if (v === '"') j = stringEnd(i, ending);
      else {
        j = i;
        while (j < ending && !/[,\]}%\s]/.test(s[j])) j++;
        j--;
      }

      result.push(fmt(i, j, d));
      i = j + 1;

      while (i < ending && /\s/.test(s[i])) i++;
      if (s[i] === ",") {
        result.push(",");
        i++;
      }
    }

    if (result.length > 1) result.push("\n" + indent.repeat(depth));
    result.push(s[ending]);
    return result.join("");
  }
  return fmt(0, n - 1, 0);
}
