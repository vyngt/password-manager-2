use leptos::prelude::*;

use ui::components::Button;
use ui::components::table::{Table, TableBody, TableCell, TableColumn, TableHeader, TableRow};
use ui::primitives::tokens::Variant;

#[component]
pub fn VPage() -> impl IntoView {
    view! {
        <div class="h-full">
            <Table>
                <TableHeader class="bg-secondary/20">
                    <TableRow>
                        <TableColumn class="p-2 hover:bg-secondary/30 transition-colors">Username</TableColumn>
                        <TableColumn class="p-2 hover:bg-secondary/30 transition-colors">Action</TableColumn>
                    </TableRow>
                </TableHeader>
                <TableBody>
                    <TableRow>
                        <TableCell class="font-jetbrains-mono">"Hello world"</TableCell>
                        <TableCell>
                            <Button variant={Variant::Text}>"Edit"</Button>
                            <Button variant={Variant::Text}>"Delete"</Button>
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell class="font-yomogi">"Cooked cooked"</TableCell>
                        <TableCell>
                            <Button variant={Variant::Text}>"Edit"</Button>
                            <Button variant={Variant::Text}>"Delete"</Button>
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell class="font-patrick-hand">"Cooked cooked"</TableCell>
                        <TableCell>
                            <Button variant={Variant::Text}>"Edit"</Button>
                            <Button variant={Variant::Text}>"Delete"</Button>
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell class="font-playwrite-de-grund">"Cooked cooked"</TableCell>
                        <TableCell>
                            <Button variant={Variant::Text}>"Edit"</Button>
                            <Button variant={Variant::Text}>"Delete"</Button>
                        </TableCell>
                    </TableRow>
                    <TableRow>
                        <TableCell class="font-edu-qld-hand">"Cooked cooked"</TableCell>
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
