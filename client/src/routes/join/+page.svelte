<script>
	// import guid checker from params dir
	import { check_guid } from '$lib/params/guid';
	let is_invalid_guid = true;
	let value = '';
	function submit() {
		if (check_guid(value)) {
			// if guid is valid, navigate to the game
			window.location.href = '/' + value;
		} else {
			// if guid is invalid, alert the user
			alert('Invalid GUID');
		}
	}

	function test_change() {
		is_invalid_guid = !check_guid(value);
		console.log({ is_invalid_guid });
	}
</script>

<form on:submit={submit}>
	<input on:change={test_change} {value} />
	<button type="submit" disabled={is_invalid_guid}>Join</button>
</form>
