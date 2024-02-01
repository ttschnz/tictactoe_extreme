// returns true if the given string is a valid GUID
export function check_guid(value: string):boolean{
    return /(?:[0-9A-Fa-f]{8})-(?:[0-9A-Fa-f]{4})-(?:[0-9A-Fa-f]{4})-(?:[0-9A-Fa-f]{4})-(?:[0-9A-Fa-f]{12})/gsy.test(value);
}
