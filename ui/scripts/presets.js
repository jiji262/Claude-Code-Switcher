/* ==========================================================================
   Claude Code Switcher v2.0 — Provider Presets & Bidirectional Schema Serializer
   ========================================================================== */

window.CCS_PRESETS = [
  {
    id: "anthropic",
    name: "Anthropic Official",
    label: "Anthropic 官方",
    host: "api.anthropic.com",
    description: "Claude 官方 API 或 Claude Max 订阅直连",
    template: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.anthropic.com",
        ANTHROPIC_API_KEY: ""
      },
      model: "claude-sonnet-4-6"
    }
  },
  {
    id: "openrouter",
    name: "OpenRouter",
    label: "OpenRouter 网关",
    host: "openrouter.ai",
    description: "OpenRouter 原生 Anthropic Messages 兼容网关",
    template: {
      env: {
        ANTHROPIC_BASE_URL: "https://openrouter.ai/api",
        ANTHROPIC_AUTH_TOKEN: ""
      },
      model: "anthropic/claude-sonnet-4.6"
    }
  },
  {
    id: "deepseek",
    name: "DeepSeek",
    label: "DeepSeek 兼容端",
    host: "api.deepseek.com",
    description: "DeepSeek 官方 Anthropic API 兼容层",
    template: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.deepseek.com/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "deepseek-chat",
        ANTHROPIC_SMALL_FAST_MODEL: "deepseek-chat",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1"
      }
    }
  },
  {
    id: "glm",
    name: "Zhipu GLM",
    label: "智谱 GLM Coding",
    host: "open.bigmodel.cn",
    description: "智谱开放平台 Anthropic 协议编码网关",
    template: {
      env: {
        ANTHROPIC_BASE_URL: "https://open.bigmodel.cn/api/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1"
      }
    }
  },
  {
    id: "kimi",
    name: "Moonshot Kimi",
    label: "Kimi (月之暗面)",
    host: "api.moonshot.cn",
    description: "Moonshot Kimi Anthropic 兼容端点",
    template: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.moonshot.cn/anthropic",
        ANTHROPIC_AUTH_TOKEN: "",
        CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC: "1"
      }
    }
  },
  {
    id: "siliconflow",
    name: "SiliconFlow",
    label: "硅基流动",
    host: "api.siliconflow.cn",
    description: "SiliconFlow 高速推理兼容网关",
    template: {
      env: {
        ANTHROPIC_BASE_URL: "https://api.siliconflow.cn/",
        ANTHROPIC_AUTH_TOKEN: "",
        ANTHROPIC_MODEL: "Pro/deepseek-ai/DeepSeek-V3"
      }
    }
  },
  {
    id: "bedrock",
    name: "AWS Bedrock",
    label: "AWS Bedrock",
    host: "bedrock.aws",
    description: "企业级 AWS Bedrock 托管 Claude 模型",
    template: {
      env: {
        CLAUDE_CODE_USE_BEDROCK: "1",
        AWS_REGION: "us-east-1",
        ANTHROPIC_MODEL: "us.anthropic.claude-sonnet-4-6-v1:0"
      }
    }
  },
  {
    id: "vertex",
    name: "Google Vertex AI",
    label: "Google Vertex AI",
    host: "vertex.googleapis.com",
    description: "GCP Vertex AI 企业托管 Claude 环境",
    template: {
      env: {
        CLAUDE_CODE_USE_VERTEX: "1",
        CLOUD_ML_REGION: "global",
        ANTHROPIC_VERTEX_PROJECT_ID: ""
      }
    }
  },
  {
    id: "custom",
    name: "Custom Proxy",
    label: "自建中转 / LiteLLM",
    host: "custom-gateway",
    description: "支持 OneAPI / NewAPI / LiteLLM 自定义中转",
    template: {
      env: {
        ANTHROPIC_BASE_URL: "https://your-gateway.example.com",
        ANTHROPIC_AUTH_TOKEN: ""
      }
    }
  }
];

window.CCS_KNOWN_ENV_KEYS = new Set([
  "ANTHROPIC_BASE_URL",
  "ANTHROPIC_API_KEY",
  "ANTHROPIC_AUTH_TOKEN",
  "ANTHROPIC_MODEL",
  "ANTHROPIC_DEFAULT_OPUS_MODEL",
  "ANTHROPIC_DEFAULT_SONNET_MODEL",
  "ANTHROPIC_DEFAULT_HAIKU_MODEL",
  "ANTHROPIC_SMALL_FAST_MODEL",
  "CLAUDE_CODE_MAX_OUTPUT_TOKENS",
  "CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC",
  "CLAUDE_CODE_USE_BEDROCK",
  "CLAUDE_CODE_USE_VERTEX",
  "AWS_REGION",
  "CLOUD_ML_REGION",
  "ANTHROPIC_VERTEX_PROJECT_ID",
  "HTTPS_PROXY",
  "HTTP_PROXY"
]);

window.CCS_PARSE_FORM = function (jsonText) {
  let obj;
  try {
    obj = JSON.parse(jsonText || "{}");
    if (!obj || typeof obj !== "object" || Array.isArray(obj)) obj = {};
  } catch (err) {
    return { valid: false, error: err.message };
  }

  const env = (obj.env && typeof obj.env === "object" && !Array.isArray(obj.env)) ? obj.env : {};
  const perms = (obj.permissions && typeof obj.permissions === "object") ? obj.permissions : {};

  let authMode = "oauth_none";
  if (String(env.CLAUDE_CODE_USE_BEDROCK || "") === "1") {
    authMode = "bedrock";
  } else if (String(env.CLAUDE_CODE_USE_VERTEX || "") === "1") {
    authMode = "vertex";
  } else if (env.ANTHROPIC_AUTH_TOKEN !== undefined && env.ANTHROPIC_AUTH_TOKEN !== "") {
    authMode = "auth_token";
  } else if (env.ANTHROPIC_API_KEY !== undefined && env.ANTHROPIC_API_KEY !== "") {
    authMode = "api_key";
  } else if (env.ANTHROPIC_AUTH_TOKEN !== undefined) {
    authMode = "auth_token";
  } else if (env.ANTHROPIC_API_KEY !== undefined) {
    authMode = "api_key";
  }

  const extraEnv = [];
  for (const [k, v] of Object.entries(env)) {
    if (!window.CCS_KNOWN_ENV_KEYS.has(k)) {
      extraEnv.push({ key: k, value: String(v ?? "") });
    }
  }

  return {
    valid: true,
    raw: obj,
    baseUrl: String(env.ANTHROPIC_BASE_URL ?? ""),
    authMode,
    apiKey: String(env.ANTHROPIC_API_KEY ?? ""),
    authToken: String(env.ANTHROPIC_AUTH_TOKEN ?? ""),
    httpsProxy: String(env.HTTPS_PROXY ?? env.HTTP_PROXY ?? ""),
    awsRegion: String(env.AWS_REGION ?? ""),
    vertexRegion: String(env.CLOUD_ML_REGION ?? ""),
    vertexProject: String(env.ANTHROPIC_VERTEX_PROJECT_ID ?? ""),
    topModel: String(obj.model ?? ""),
    envModel: String(env.ANTHROPIC_MODEL ?? ""),
    opusModel: String(env.ANTHROPIC_DEFAULT_OPUS_MODEL ?? ""),
    sonnetModel: String(env.ANTHROPIC_DEFAULT_SONNET_MODEL ?? ""),
    haikuModel: String(env.ANTHROPIC_DEFAULT_HAIKU_MODEL ?? ""),
    fastModel: String(env.ANTHROPIC_SMALL_FAST_MODEL ?? ""),
    maxOutputTokens: String(env.CLAUDE_CODE_MAX_OUTPUT_TOKENS ?? ""),
    disableTraffic: String(env.CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC ?? "") === "1",
    alwaysThinking: Boolean(obj.alwaysThinkingEnabled),
    defaultPermMode: String(perms.defaultMode ?? ""),
    extraEnv
  };
};

window.CCS_SERIALIZE_FORM = function (currentJsonText, patch) {
  let obj = {};
  try {
    const parsed = JSON.parse(currentJsonText || "{}");
    if (parsed && typeof parsed === "object" && !Array.isArray(parsed)) {
      obj = parsed;
    }
  } catch (_) {
    obj = {};
  }

  if (!obj.env || typeof obj.env !== "object" || Array.isArray(obj.env)) {
    obj.env = {};
  }

  const setOrDeleteEnv = (k, val) => {
    if (val === undefined || val === null || val === "") {
      delete obj.env[k];
    } else {
      obj.env[k] = String(val);
    }
  };

  if ("baseUrl" in patch) setOrDeleteEnv("ANTHROPIC_BASE_URL", patch.baseUrl.trim());
  if ("httpsProxy" in patch) {
    const p = patch.httpsProxy.trim();
    setOrDeleteEnv("HTTPS_PROXY", p);
    if (!p) delete obj.env.HTTP_PROXY;
  }

  if ("authMode" in patch || "secretValue" in patch) {
    const form = window.CCS_PARSE_FORM(JSON.stringify(obj));
    const mode = patch.authMode || form.authMode;
    const secret = ("secretValue" in patch)
      ? patch.secretValue
      : (mode === "api_key" ? form.apiKey : form.authToken);

    delete obj.env.CLAUDE_CODE_USE_BEDROCK;
    delete obj.env.CLAUDE_CODE_USE_VERTEX;

    if (mode === "api_key") {
      delete obj.env.ANTHROPIC_AUTH_TOKEN;
      obj.env.ANTHROPIC_API_KEY = secret;
    } else if (mode === "auth_token") {
      delete obj.env.ANTHROPIC_API_KEY;
      obj.env.ANTHROPIC_AUTH_TOKEN = secret;
    } else if (mode === "oauth_none") {
      delete obj.env.ANTHROPIC_API_KEY;
      delete obj.env.ANTHROPIC_AUTH_TOKEN;
    } else if (mode === "bedrock") {
      delete obj.env.ANTHROPIC_API_KEY;
      delete obj.env.ANTHROPIC_AUTH_TOKEN;
      obj.env.CLAUDE_CODE_USE_BEDROCK = "1";
    } else if (mode === "vertex") {
      delete obj.env.ANTHROPIC_API_KEY;
      delete obj.env.ANTHROPIC_AUTH_TOKEN;
      obj.env.CLAUDE_CODE_USE_VERTEX = "1";
    }
  }

  if ("awsRegion" in patch) setOrDeleteEnv("AWS_REGION", patch.awsRegion.trim());
  if ("vertexRegion" in patch) setOrDeleteEnv("CLOUD_ML_REGION", patch.vertexRegion.trim());
  if ("vertexProject" in patch) setOrDeleteEnv("ANTHROPIC_VERTEX_PROJECT_ID", patch.vertexProject.trim());

  if ("topModel" in patch) {
    const m = patch.topModel.trim();
    if (!m) delete obj.model;
    else obj.model = m;
  }
  if ("envModel" in patch) setOrDeleteEnv("ANTHROPIC_MODEL", patch.envModel.trim());
  if ("opusModel" in patch) setOrDeleteEnv("ANTHROPIC_DEFAULT_OPUS_MODEL", patch.opusModel.trim());
  if ("sonnetModel" in patch) setOrDeleteEnv("ANTHROPIC_DEFAULT_SONNET_MODEL", patch.sonnetModel.trim());
  if ("haikuModel" in patch) setOrDeleteEnv("ANTHROPIC_DEFAULT_HAIKU_MODEL", patch.haikuModel.trim());
  if ("fastModel" in patch) setOrDeleteEnv("ANTHROPIC_SMALL_FAST_MODEL", patch.fastModel.trim());
  if ("maxOutputTokens" in patch) setOrDeleteEnv("CLAUDE_CODE_MAX_OUTPUT_TOKENS", patch.maxOutputTokens.trim());
  if ("disableTraffic" in patch) {
    setOrDeleteEnv("CLAUDE_CODE_DISABLE_NONESSENTIAL_TRAFFIC", patch.disableTraffic ? "1" : "");
  }
  if ("alwaysThinking" in patch) {
    if (patch.alwaysThinking) obj.alwaysThinkingEnabled = true;
    else delete obj.alwaysThinkingEnabled;
  }
  if ("defaultPermMode" in patch) {
    const pm = patch.defaultPermMode.trim();
    if (!pm) {
      if (obj.permissions && typeof obj.permissions === "object") {
        delete obj.permissions.defaultMode;
        if (Object.keys(obj.permissions).length === 0) delete obj.permissions;
      }
    } else {
      if (!obj.permissions || typeof obj.permissions !== "object") obj.permissions = {};
      obj.permissions.defaultMode = pm;
    }
  }

  if ("extraEnv" in patch && Array.isArray(patch.extraEnv)) {
    for (const k of Object.keys(obj.env)) {
      if (!window.CCS_KNOWN_ENV_KEYS.has(k)) {
        delete obj.env[k];
      }
    }
    for (const row of patch.extraEnv) {
      const k = (row.key || "").trim();
      if (k) obj.env[k] = String(row.value ?? "");
    }
  }

  return JSON.stringify(obj, null, 2);
};
