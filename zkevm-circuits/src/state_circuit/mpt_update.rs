#[derive(Eq, PartialEq, Hash, Clone, Debug)]
enum MptKey {
    Account {
        address: Address,
        field_tag: AccountFieldTag,
    },
    AccountStorage {
        tx_id: usize,
        address: Address,
        storage_key: Word,
    },
}

#[derive(Debug, Clone)]
struct MptValue<F> {
    old_root: F,
    new_root: F,
    old_value: F,
    new_value: F,
}

impl<F: Field> MptValue<F> {
    fn new(row: &Rw, old_root: F, new_root: F, randomness: F) -> Self {
        Self {
            old_root,
            new_root,
            old_value: row.value_prev_assignment(randomness).unwrap(),
            new_value: row.value_assignment(randomness),
        }
    }
}

impl MptKey {
    fn address<F: Field>(&self) -> F {
        match self {
            Self::Account { address, .. } | Self::AccountStorage { address, .. } => {
                address.to_scalar().unwrap()
            }
        }
    }
    fn field_tag<F: Field>(&self) -> F {
        match self {
            Self::Account { field_tag, .. } => F::from(*field_tag as u64),
            Self::AccountStorage { .. } => F::zero(),
        }
    }
    fn storage_key<F: Field>(&self, randomness: F) -> F {
        match self {
            Self::Account { .. } => F::zero(),
            Self::AccountStorage { storage_key, .. } => {
                RandomLinearCombination::random_linear_combine(
                    storage_key.to_le_bytes(),
                    randomness,
                )
            }
        }
    }
}

fn mpt_key(row: &Rw) -> Option<MptKey> {
    match row {
        Rw::Account {
            account_address,
            field_tag,
            ..
        } => Some(MptKey::Account {
            address: *account_address,
            field_tag: *field_tag,
        }),
        Rw::AccountStorage {
            tx_id,
            account_address,
            storage_key,
            ..
        } => Some(MptKey::AccountStorage {
            tx_id: *tx_id,
            address: *account_address,
            storage_key: *storage_key,
        }),
        _ => None,
    }
}

fn fake_mpt_updates<F: Field>(rows: &[Rw], randomness: F) -> HashMap<MptKey, MptValue<F>> {
    rows.iter()
        .group_by(|row| mpt_key(row))
        .into_iter()
        .filter_map(|(key, rows)| key.map(|key| (key, rows)))
        .enumerate()
        .map(|(i, (key, mut rows))| {
            let first = rows.next().unwrap();
            let mut value = MptValue::new(
                first,
                F::from(i as u64),
                F::from((i + 1) as u64),
                randomness,
            );
            value.new_value = rows.last().unwrap_or(first).value_assignment(randomness);
            (key, value)
        })
        .collect()
}
