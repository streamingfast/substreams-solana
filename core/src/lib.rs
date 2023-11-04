use crate::pb::sf::solana::r#type::v1::ConfirmedTransaction;

pub mod pb;

/// Helpers to deal with block sources.
pub mod block_view;

impl ConfirmedTransaction {
    pub fn resolved_accounts(&self) -> Vec<&Vec<u8>> {
        let meta = self.meta.as_ref().unwrap();
        let mut accounts = vec![];

        self.transaction.as_ref().unwrap().message.as_ref().unwrap().account_keys.iter().for_each(|addr| {
            accounts.push(addr);
        });
        meta.loaded_writable_addresses.iter().for_each(|addr| {
            accounts.push(addr);
        });
        meta.loaded_readonly_addresses.iter().for_each(|addr| {
            accounts.push(addr);
        });

        return accounts;
    }

    pub fn resolved_accounts_as_strings(&self) -> Vec<String> {
        let accounts = self.resolved_accounts();

        let mut resolved_accounts = vec![];

        accounts
            .iter()
            .for_each(|addr| resolved_accounts.push(bs58::encode(addr).into_string()));

        return resolved_accounts;
    }
}
