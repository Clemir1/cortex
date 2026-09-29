# experiments/

Um diretório por experimento, com layout fixo:
- manifest.toml: metadados e status do experimento.
- hypothesis.md: hipótese e critério pré-registrado.
- config.toml: delta sobre config/default.toml.
- expected_metrics.toml: métricas e critérios numéricos.

Resultado SEMPRE em var/runs/<run_id>/ — nunca no diretório do experimento.
Veredito PRÉ-registrado: decisão antes da observação; sem calibração retroativa.
Promoção de qualquer hipótese exige ADR.
Copie _template/ para iniciar um novo experimento.
