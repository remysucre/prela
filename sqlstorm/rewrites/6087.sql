WITH RankedPosts AS (
    SELECT 
        p.Id AS PostId,
        p.Title,
        p.ViewCount,
        p.CreationDate,
        p.OwnerUserId,
        u.DisplayName AS OwnerDisplayName,
        ROW_NUMBER() OVER (PARTITION BY p.OwnerUserId ORDER BY p.ViewCount DESC, p.Id) AS OwnerRank
    FROM 
        Posts p
    JOIN 
        Users u ON p.OwnerUserId = u.Id
    WHERE 
        p.PostTypeId = 1
        AND p.CreationDate >= cast('2024-10-01 12:34:56' as timestamp) - INTERVAL '1 year'
),
BestPosts AS (
    SELECT 
        rp.PostId,
        rp.Title,
        rp.ViewCount,
        rp.CreationDate,
        rp.OwnerUserId,
        rp.OwnerDisplayName
    FROM 
        RankedPosts rp
    WHERE 
        rp.OwnerRank <= 5
),
PostVoteStats AS (
    SELECT 
        p.Id AS PostId,
        COUNT(CASE WHEN v.VoteTypeId = 2 THEN 1 END) AS UpVotes,
        COUNT(CASE WHEN v.VoteTypeId = 3 THEN 1 END) AS DownVotes,
        COUNT(v.Id) AS TotalVotes
    FROM 
        Posts p
    LEFT JOIN 
        Votes v ON p.Id = v.PostId
    WHERE 
        p.Id IN (SELECT PostId FROM BestPosts)
    GROUP BY 
        p.Id
)
SELECT 
    bp.Title,
    bp.ViewCount,
    bp.CreationDate,
    bp.OwnerDisplayName,
    pvs.UpVotes,
    pvs.DownVotes,
    pvs.TotalVotes
FROM 
    BestPosts bp
JOIN 
    PostVoteStats pvs ON bp.PostId = pvs.PostId
ORDER BY 
    pvs.UpVotes DESC, pvs.TotalVotes DESC, bp.PostId
LIMIT 10;