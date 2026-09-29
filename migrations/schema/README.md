# migrations/schema

Evolução de schema versionada, registrada em schemas/version.
Cada mudança ganha entrada com versão, data e diff do formato.
Checkpoints sem lineage são rejeitados sem exceção.
O histórico permite comparar e reverter estados antigos.
Mudança de schema sem entrada aqui é considerada inválida.
