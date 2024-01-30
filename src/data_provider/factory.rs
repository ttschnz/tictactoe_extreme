use crate::DataProvider;

pub struct DataProviderFactory;

impl DataProviderFactory {
    pub fn create<T: DataProvider>(args: T::Args) -> Result<T, T::ErrorKind> {
        T::new(args)
    }
}
