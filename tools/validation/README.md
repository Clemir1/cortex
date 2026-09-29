# tools/validation

Validação dos schemas declarados em schemas/.
Valida contratos de eventos: campos, tipos e ordem.
Checa invariantes de checkpoint: lineage completo e hashes consistentes.
Falha rápida: checkpoint sem lineage não passa daqui.
Rodado em CI e antes de qualquer promoção de checkpoint.
