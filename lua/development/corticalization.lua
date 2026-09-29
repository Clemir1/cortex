-- corticalization.lua — critério de migração para hardware.
-- Espelho SIMPLIFICADO do critério Rust: migra só quando insustentável.
-- Função pura: retorna (migrate: boolean, reason: string).

local C = {}

-- Deve migrar? Sustentável em software -> não migra.
function C.should_migrate(energy, stability, capacity)
  energy = energy or 0
  stability = stability or 0
  capacity = capacity or 0

  -- Sustentável: energia alta mantém em software.
  if energy >= 0.8 then
    return false, "EnergySustain"
  end

  -- Sustentável: estabilidade alta dispensa migração.
  if stability >= 0.95 then
    return false, "StabilitySustain"
  end

  -- Sustentável: capacidade atual suficiente.
  if capacity >= 0.95 then
    return false, "CapacitySustain"
  end

  -- Nada sustenta em software: migra.
  return true, "Unsustained"
end

return C
