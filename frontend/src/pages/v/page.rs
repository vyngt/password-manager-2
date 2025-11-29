use leptos::prelude::*;

use ui::components::Button;
use ui::components::table::{Table, TableBody, TableCell, TableColumn, TableHeader, TableRow};
use ui::primitives::tokens::Variant;

#[component]
pub fn VPage() -> impl IntoView {
    view! {
        <div class="h-full">
            <Table>
                <TableHeader>
                    <TableColumn>Username</TableColumn>
                    <TableColumn>Password</TableColumn>
                    <TableColumn>Action</TableColumn>
                </TableHeader>
                <TableBody>
                    <TableRow>
                        <TableCell>user1</TableCell>
                        <TableCell>password1</TableCell>
                        <TableCell>
                            <Button variant={Variant::Text}>"Edit"</Button>
                            <Button variant={Variant::Text}>"Delete"</Button>
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell>user2</TableCell>
                        <TableCell>password2</TableCell>
                        <TableCell>
                            <Button variant={Variant::Text}>"Edit"</Button>
                            <Button variant={Variant::Text}>"Delete"</Button>
                        </TableCell>
                    </TableRow>
                </TableBody>
            </Table>

        </div>
    }
}
