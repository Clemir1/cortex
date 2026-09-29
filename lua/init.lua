-- init.lua — ponto de entrada das políticas Lua do Triad_AEE.
-- Sandbox por projeto: NENHUM require, NENHUM os/io/debug.
-- Lua é APENAS política: nunca escreve estado, nunca age.

-- Registro local: nome -> função de política.
local registry = {}

-- Global lida pelo host Rust (crate triad-bindings-lua).
triad = {
  version = "0.1.0",

  -- Falhas de registro ficam aqui; o host nunca cai.
  errors = {},

  -- Registra política com wrap em pcall.
  register_policy = function(name, fn)
    local ok, err = pcall(function()
      -- Validação estrita do par recebido.
      if type(name) ~= "string" or #name == 0 then
        error("nome de política inválido")
      end
      if type(fn) ~= "function" then
        error("valor de política não é função")
      end
      if registry[name] ~= nil then
        error("política já registrada: " .. name)
      end
      registry[name] = fn
    end)

    -- Erro vira registro em triad.errors, não exceção no host.
    if not ok then
      triad.errors[#triad.errors + 1] = {
        policy = tostring(name),
        message = tostring(err),
      }
    end
  end,

  -- Tabela de leitura: o host consulta; não escreve aqui.
  policies = registry,
}
