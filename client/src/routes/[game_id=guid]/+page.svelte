<script lang="ts">
	import { page } from '$app/stores';

	// get the guid from the url

	let guid = $page.params.game_id;
</script>

<h1>{guid}</h1>
