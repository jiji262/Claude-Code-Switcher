/* ==========================================================================
   Claude Code Switcher v2.0 — Line-by-Line LCS Diff & Secret Redactor
   ========================================================================== */

window.CCS_MASK_SECRETS_IN_JSON = function (jsonText) {
  if (!jsonText) return "";
  return jsonText.replace(
    /("(?:ANTHROPIC_API_KEY|ANTHROPIC_AUTH_TOKEN|OPENAI_API_KEY|API_KEY|TOKEN|SECRET)"\s*:\s*")([^"]+)(")/gi,
    (_, p1, secret, p3) => {
      if (secret.length <= 8) return `${p1}••••••••${p3}`;
      return `${p1}${secret.slice(0, 6)}••••••${secret.slice(-4)}${p3}`;
    }
  );
};

window.CCS_COMPUTE_DIFF = function (beforeStr, afterStr) {
  const a = (beforeStr || "{}").split("\n");
  const b = (afterStr || "{}").split("\n");
  const n = a.length;
  const m = b.length;

  // Guard for very large files: fallback simple comparison above 600 lines
  if (n * m > 360000) {
    const out = [];
    const maxLen = Math.max(n, m);
    for (let i = 0; i < maxLen; i++) {
      if (a[i] === b[i]) {
        out.push({ type: "ctx", oldLn: i + 1, newLn: i + 1, text: a[i] });
      } else {
        if (i < n) out.push({ type: "del", oldLn: i + 1, newLn: "", text: a[i] });
        if (i < m) out.push({ type: "add", oldLn: "", newLn: i + 1, text: b[i] });
      }
    }
    return out;
  }

  const dp = Array.from({ length: n + 1 }, () => new Uint16Array(m + 1));
  for (let i = n - 1; i >= 0; i--) {
    for (let j = m - 1; j >= 0; j--) {
      if (a[i] === b[j]) {
        dp[i][j] = dp[i + 1][j + 1] + 1;
      } else {
        dp[i][j] = dp[i + 1][j] >= dp[i][j + 1] ? dp[i + 1][j] : dp[i][j + 1];
      }
    }
  }

  const ops = [];
  let i = 0;
  let j = 0;
  let oldLn = 1;
  let newLn = 1;
  while (i < n && j < m) {
    if (a[i] === b[j]) {
      ops.push({ type: "ctx", oldLn: oldLn++, newLn: newLn++, text: a[i] });
      i++;
      j++;
    } else if (dp[i + 1][j] >= dp[i][j + 1]) {
      ops.push({ type: "del", oldLn: oldLn++, newLn: "", text: a[i] });
      i++;
    } else {
      ops.push({ type: "add", oldLn: "", newLn: newLn++, text: b[j] });
      j++;
    }
  }
  while (i < n) {
    ops.push({ type: "del", oldLn: oldLn++, newLn: "", text: a[i++] });
  }
  while (j < m) {
    ops.push({ type: "add", oldLn: "", newLn: newLn++, text: b[j++] });
  }
  return ops;
};
