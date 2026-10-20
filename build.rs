// манифест с правами админа нужен только для windows-сборок
fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        embed_manifest::embed_manifest(
            embed_manifest::new_manifest("diddy slave")
                .requested_execution_level(embed_manifest::manifest::ExecutionLevel::AsInvoker),
        )
        .expect("не удалось вшить манифест");
    }
}
