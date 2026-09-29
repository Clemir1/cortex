-- ecology.lua — ecologia de módulos: coexistência e nichos.
-- Matriz declarativa e hash determinístico; sem estado.

local E = {}

-- Duplas incompatíveis (matriz simétrica e pequena).
local INCOMPATIBLE = {
  predator      = { prey_parallel = true },
  prey_parallel = { predator = true },
  parasite      = { host_critical = true },
  host_critical = { parasite = true },
}

-- Pode coexistir? Par declarado -> false; senão true.
function E.can_coexist(a, b)
  local row = INCOMPATIBLE[a]
  if row == nil then
    return true
  end
  return row[b] ~= true
end

-- Nicho determinístico: soma de bytes do nome % 256 (0..255).
function E.niche(name)
  name = name or ""
  local sum = 0
  for i = 1, #name do
    sum = sum + string.byte(name, i)
  end
  return sum % 256
end

return E
