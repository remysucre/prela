
WITH PostMetrics AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.Body,
        p.CreationDate,
        p.ViewCount,
        p.AnswerCount,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 2 THEN 1 ELSE 0 END), 0) AS UpVotes,
        COALESCE(SUM(CASE WHEN v.VoteTypeId = 3 THEN 1 ELSE 0 END), 0) AS DownVotes,
        STRING_AGG(t.TagName, ', ' ORDER BY t.TagName) AS Tags
    FROM 
        Posts p
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    LEFT JOIN 
        LATERAL (
            SELECT unnest(string_to_array(substring(p.Tags, 2, length(p.Tags)-2), '><')) AS TagName
        ) t ON TRUE
    WHERE 
        p.CreationDate >= TIMESTAMP '2024-10-01 12:34:56' - INTERVAL '1 year' AND 
        p.PostTypeId = 1 
    GROUP BY 
        p.Id, p.Title, p.Body, p.CreationDate, p.ViewCount, p.AnswerCount
),
HighEngagementPosts AS (
    SELECT 
        pm.PostId,
        pm.Title,
        pm.ViewCount,
        pm.AnswerCount,
        pm.UpVotes,
        pm.DownVotes,
        pm.Tags,
        (pm.UpVotes - pm.DownVotes) AS NetVotes,
        RANK() OVER (ORDER BY pm.ViewCount DESC, (pm.UpVotes - pm.DownVotes) DESC) AS EngagementRank
    FROM 
        PostMetrics pm
    WHERE 
        pm.ViewCount > 50 
)

SELECT 
    he.PostId,
    he.Title,
    he.ViewCount,
    he.AnswerCount,
    he.UpVotes,
    he.DownVotes,
    he.NetVotes,
    he.Tags,
    u.DisplayName AS OwnerDisplayName,
    u.Reputation AS OwnerReputation,
    p.LastEditDate
FROM 
    HighEngagementPosts he
JOIN 
    Posts p ON he.PostId = p.Id
JOIN 
    Users u ON p.OwnerUserId = u.Id
WHERE 
    he.EngagementRank <= 10 
ORDER BY 
    he.EngagementRank;
