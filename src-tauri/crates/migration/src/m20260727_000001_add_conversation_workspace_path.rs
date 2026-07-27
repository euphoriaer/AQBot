use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if !manager
            .has_column("conversations", "workspace_path")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(Conversations::Table)
                        .add_column(
                            ColumnDef::new(Conversations::WorkspacePath)
                                .string()
                                .null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        if !manager
            .has_column("conversation_categories", "default_workspace_path")
            .await?
        {
            manager
                .alter_table(
                    Table::alter()
                        .table(ConversationCategories::Table)
                        .add_column(
                            ColumnDef::new(ConversationCategories::DefaultWorkspacePath)
                                .string()
                                .null(),
                        )
                        .to_owned(),
                )
                .await?;
        }

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(ConversationCategories::Table)
                    .drop_column(ConversationCategories::DefaultWorkspacePath)
                    .to_owned(),
            )
            .await?;

        manager
            .alter_table(
                Table::alter()
                    .table(Conversations::Table)
                    .drop_column(Conversations::WorkspacePath)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Conversations {
    Table,
    WorkspacePath,
}

#[derive(DeriveIden)]
enum ConversationCategories {
    Table,
    DefaultWorkspacePath,
}
