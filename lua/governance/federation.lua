-- federation.lua — fases da federação.
-- O Rust CONDUZ o protocolo; o Lua apenas opina.
-- Biblioteca pura: sem registro de política, sem estado.

local F = {}

-- Sequência ESTRITA do ciclo federativo.
local PHASES = {
  "need",
  "proposal",
  "agreement",
  "resource_transfer",
  "effect",
  "settlement",
}

-- Próxima fase da sequência; fecha o ciclo em "need".
function F.next_phase(phase)
  for i, name in ipairs(PHASES) do
    if name == phase then
      local j = i + 1
      if j > #PHASES then
        j = 1
      end
      return PHASES[j]
    end
  end
  -- Fase desconhecida: reinicia o ciclo (determinístico).
  return "need"
end

-- Transferência só com recursos mínimos e confiança suficiente.
function F.should_transfer(resources, trust)
  resources = resources or 0
  trust = trust or 0
  return resources >= 1 and trust >= 0.5
end

return F
