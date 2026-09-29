-- energy.lua — política O1 de homeostase energética.
-- Lê context.mean_energy e ajusta a taxa de intake do substrato.
-- Determinística: sem estado, sem tempo, sem aleatoriedade.

-- Setpoint O1 e banda morta do loop homeostático.
local SETPOINT = 0.8
local BAND_LOW = 0.75
local BAND_HIGH = 0.85

triad.register_policy("energy", function(context)
  context = context or {}

  -- Default 0.5 quando o sinal está ausente.
  local mean = context.mean_energy or 0.5

  -- Banda morta: energia estável, não propõe.
  if mean >= BAND_LOW and mean <= BAND_HIGH then
    return nil
  end

  local value
  local reason
  if mean < SETPOINT then
    -- Déficit: intake proporcional à distância do alvo.
    value = 0.5 + (SETPOINT - mean) * 1.5
    reason = "déficit de energia contra o setpoint O1"
  else
    -- Excedente: reduz intake proporcionalmente.
    value = 0.5 - (mean - SETPOINT) * 1.5
    reason = "excedente de energia sobre o setpoint O1"
  end

  -- Clamp no intervalo 0.0..1.5.
  if value < 0.0 then value = 0.0 end
  if value > 1.5 then value = 1.5 end

  return {
    target_module = "l1.substrate",
    target_parameter = "energy.intake_rate",
    value = value,
    reason = reason,
    confidence = 0.8,
    ttl = 10,
  }
end)
