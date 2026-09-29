-- regeneration.lua — prioridade de cura por severidade.
-- Ordena uma CÓPIA; a entrada nunca é mutada. Sem estado.

local R = {}

-- Severidade por tipo de dano (declarativo).
local SEVERITY = {
  overload = 4,
  corruption = 3,
  starvation = 2,
  disconnection = 1,
}

-- Severidade de um dano; desconhecido = 0 (fundo da fila).
local function severity_of(damage)
  return SEVERITY[damage] or 0
end

-- Cópia ordenada por severidade DESCENDENTE (maior primeiro).
function R.heal_priority(damages)
  -- Entrada protegida: não-tabela vira lista vazia.
  local source = {}
  if type(damages) == "table" then
    source = damages
  end

  -- Cópia rasa: entrada permanece intacta.
  local copy = {}
  for i = 1, #source do
    copy[i] = source[i]
  end

  -- Comparador puro: severidade maior antes.
  table.sort(copy, function(a, b)
    return severity_of(a) > severity_of(b)
  end)

  return copy
end

return R
