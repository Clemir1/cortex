# migrations/state

Mapa old→new do estado: do modelo Python para latest→checkpoints com lineage.
Cada checkpoint carrega parent_run_id, checkpoint_hash, state_hash e schema_version.
O estado Python será exportado como arquivo serializado, único e versionado.
A reidratação valida hashes e lineage antes de aceitar qualquer estado.
Estado sem lineage válido é rejeitado, sem exceção.
O formato exportado serve de ponte entre o legado e o novo núcleo.
