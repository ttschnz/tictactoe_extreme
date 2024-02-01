import {check_guid} from '$lib/params/guid';

export function match(value: string):boolean{
    return check_guid(value);
}