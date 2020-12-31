use diesel::prelude::*;
use diesel::pg::Pg;
use diesel::query_builder::*;
use diesel::expression::{Expression, NonAggregate};
use diesel::sql_types::{Text, Bool};

#[macro_export]
#[doc(hidden)]
macro_rules! impl_selectable_expression {
    ($struct_name:ident) => {
        impl_selectable_expression!(ty_params = (), struct_ty = $struct_name,);
    };

    ($struct_name:ident<$($ty_params:ident),+>) => {
        impl_selectable_expression!(
            ty_params = ($($ty_params),+),
            struct_ty = $struct_name<$($ty_params),+>,
        );
    };

    (ty_params = ($($ty_params:ident),*), struct_ty = $struct_ty:ty,) => {
        impl<$($ty_params,)* QS> diesel::expression::SelectableExpression<QS>
            for $struct_ty where
                $struct_ty: diesel::expression::AppearsOnTable<QS>,
                $($ty_params: diesel::expression::SelectableExpression<QS>,)*
        {
        }

        impl<$($ty_params,)* QS> diesel::expression::AppearsOnTable<QS>
            for $struct_ty where
                $struct_ty: diesel::expression::Expression,
                $($ty_params: diesel::expression::AppearsOnTable<QS>,)*
        {
        }
    };
}

#[derive(Debug, Clone, QueryId)]
pub struct CryptEqOp<T> {
    pub column: T,
    pub test: String
}

impl<T> CryptEqOp<T> {
    pub fn new(column: T, test: String) -> Self {
        CryptEqOp {
            column,
            test
        }
    }
}

impl_selectable_expression!(CryptEqOp<T>);

impl<T> Expression for CryptEqOp<T> {
    type SqlType = Bool;
}

impl<T> NonAggregate for CryptEqOp<T> {}

impl<T> QueryFragment<Pg> for CryptEqOp<T>
where
    T: QueryFragment<Pg>
{
    fn walk_ast(&self, mut out: AstPass<Pg>) -> QueryResult<()> {
        self.column.walk_ast(out.reborrow())?;
        out.push_sql(" = crypt(");
        out.push_bind_param::<Text, _>(&self.test)?;
        out.push_sql(", ");
        self.column.walk_ast(out.reborrow())?;
        out.push_sql(")");
        Ok(())
    }
}

pub type CryptEq<T> = CryptEqOp<T>;

pub trait CryptExpressionMethods
where
    Self: Expression<SqlType = Text> + Sized,
{
    fn crypt_eq(self, test: &String) -> CryptEq<Self> {
        CryptEq::new(self, test.to_string())
    }
}

impl<T: Expression<SqlType = Text>> CryptExpressionMethods for T {}